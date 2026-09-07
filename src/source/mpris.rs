use crate::options::{Control, SourceKind};
use std::sync::mpsc::{self, Receiver};

#[cfg(target_os = "linux")]
pub(super) fn control(kind: SourceKind, command: Control) -> Result<(), String> {
    let bus = Connection::new_session().map_err(|error| error.to_string())?;
    let proxy = bus.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        Duration::from_secs(2),
    );
    let (names,): (Vec<String>,) = proxy
        .method_call("org.freedesktop.DBus", "ListNames", ())
        .map_err(|error| error.to_string())?;
    let mut players = names
        .into_iter()
        .filter(|name| super::player_source(name) == Some(kind));
    let name = players.next().ok_or("no matching player")?;
    if players.next().is_some() {
        return Err("multiple matching players; refusing ambiguous control".to_owned());
    }
    let proxy = bus.with_proxy(name, "/org/mpris/MediaPlayer2", Duration::from_secs(2));
    let method = match command {
        Control::Play => "Play",
        Control::Pause => "Pause",
        Control::Toggle => "PlayPause",
        Control::Previous => "Previous",
        Control::Next => "Next",
    };
    proxy
        .method_call::<(), _, _, _>(PLAYER_INTERFACE, method, ())
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn control(_kind: SourceKind, _command: Control) -> Result<(), String> {
    Err("MPRIS requires Linux".to_owned())
}

#[cfg(target_os = "linux")]
use {
    dbus::{arg::PropMap, blocking::Connection, message::MatchRule},
    std::{collections::HashMap, sync::mpsc::SyncSender, time::Duration},
};

pub(super) enum Event {
    Activity(String),
    Unavailable(String),
}

pub(super) struct Events {
    receiver: Receiver<Event>,
    #[cfg(target_os = "linux")]
    _cancel: Option<std::os::unix::net::UnixStream>,
}

impl Events {
    pub(super) fn try_recv(&self) -> Result<Event, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }
}

pub(super) fn subscribe() -> Events {
    let (sender, events) = mpsc::sync_channel(32);
    let mut events = Events {
        receiver: events,
        #[cfg(target_os = "linux")]
        _cancel: None,
    };
    #[cfg(target_os = "linux")]
    {
        let (cancel, stopped) = match std::os::unix::net::UnixStream::pair() {
            Ok(pair) => pair,
            Err(error) => {
                let _ = sender.send(Event::Unavailable(error.to_string()));
                return events;
            }
        };
        events._cancel = Some(cancel);
        if let Err(error) = std::thread::Builder::new()
            .name("mpris-events".to_owned())
            .spawn({
                let sender = sender.clone();
                move || {
                    if let Err(error) = watch(&sender, &stopped) {
                        let _ = sender.send(Event::Unavailable(error.to_string()));
                    }
                }
            })
        {
            let _ = sender.send(Event::Unavailable(error.to_string()));
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = sender.send(Event::Unavailable("MPRIS requires Linux".to_owned()));
    events
}

#[cfg(target_os = "linux")]
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[cfg(target_os = "linux")]
fn watch(
    sender: &SyncSender<Event>,
    stopped: &std::os::unix::net::UnixStream,
) -> Result<(), dbus::Error> {
    use std::os::fd::AsRawFd;
    let is_player = |name: &str| name.starts_with("org.mpris.MediaPlayer2.");
    let mut channel = dbus::channel::Channel::get_private(dbus::channel::BusType::Session)?;
    channel.set_watch_enabled(true);
    let bus = Connection::from(channel);
    let owner_rule = MatchRule::new_signal("org.freedesktop.DBus", "NameOwnerChanged")
        .with_sender("org.freedesktop.DBus");
    let player_rule = MatchRule::new_signal("org.freedesktop.DBus.Properties", "PropertiesChanged")
        .with_path("/org/mpris/MediaPlayer2");
    bus.add_match_no_cb(&format!(
        "{},arg0namespace='org.mpris.MediaPlayer2'",
        owner_rule.match_str()
    ))?;
    bus.add_match_no_cb(&format!(
        "{},arg0='{PLAYER_INTERFACE}'",
        player_rule.match_str()
    ))?;

    let proxy = bus.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        Duration::from_secs(2),
    );
    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ())?;
    let mut owners = HashMap::new();
    for name in names.into_iter().filter(|name| is_player(name)) {
        if let Ok((owner,)) = proxy.method_call::<(String,), _, _, _>(
            "org.freedesktop.DBus",
            "GetNameOwner",
            (name.as_str(),),
        ) {
            if sender.send(Event::Activity(name.clone())).is_err() {
                return Ok(());
            }
            owners.insert(name, owner);
        }
    }

    loop {
        while let Some(message) = bus.channel().pop_message() {
            if owner_rule.matches(&message) {
                let Ok((name, _, owner)) = message.read3::<String, String, String>() else {
                    continue;
                };
                if !is_player(&name) {
                    continue;
                }
                if owner.is_empty() {
                    owners.remove(&name);
                } else {
                    owners.insert(name.clone(), owner);
                    if sender.send(Event::Activity(name)).is_err() {
                        return Ok(());
                    }
                }
            } else if player_rule.matches(&message) {
                let Ok((interface, properties, invalidated)) =
                    message.read3::<String, PropMap, Vec<String>>()
                else {
                    continue;
                };
                if interface == PLAYER_INTERFACE
                    && ["PlaybackStatus", "Metadata"].iter().any(|key| {
                        properties.contains_key(*key) || invalidated.iter().any(|name| name == key)
                    })
                {
                    for (name, owner) in &owners {
                        if message.sender().is_some_and(|sender| owner == &*sender)
                            && sender.send(Event::Activity(name.clone())).is_err()
                        {
                            return Ok(());
                        }
                    }
                }
            }
        }
        let watch = bus.channel().watch();
        let mut fds = [
            libc::pollfd {
                fd: watch.fd,
                events: (if watch.read { libc::POLLIN } else { 0 })
                    | (if watch.write { libc::POLLOUT } else { 0 }),
                revents: 0,
            },
            libc::pollfd {
                fd: stopped.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, -1) } < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(dbus::Error::new_failed(&error.to_string()));
        }
        if fds[1].revents != 0 {
            return Ok(());
        }
        bus.channel()
            .read_write(Some(Duration::ZERO))
            .map_err(|()| dbus::Error::new_failed("MPRIS session bus disconnected"))?;
    }
}
