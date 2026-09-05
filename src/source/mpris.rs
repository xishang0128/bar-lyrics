use std::sync::mpsc::{self, Receiver};

#[cfg(target_os = "linux")]
use {
    dbus::{arg::PropMap, blocking::Connection, message::MatchRule},
    std::{collections::HashMap, sync::mpsc::SyncSender, time::Duration},
};

pub(super) enum Event {
    Activity(String),
    Unavailable(String),
}

pub(super) fn subscribe() -> Receiver<Event> {
    let (sender, events) = mpsc::sync_channel(32);
    #[cfg(target_os = "linux")]
    if let Err(error) = std::thread::Builder::new()
        .name("mpris-events".to_owned())
        .spawn({
            let sender = sender.clone();
            move || {
                if let Err(error) = watch(&sender) {
                    let _ = sender.send(Event::Unavailable(error.to_string()));
                }
            }
        })
    {
        let _ = sender.send(Event::Unavailable(error.to_string()));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = sender.send(Event::Unavailable("MPRIS requires Linux".to_owned()));
    events
}

#[cfg(target_os = "linux")]
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";

#[cfg(target_os = "linux")]
fn watch(sender: &SyncSender<Event>) -> Result<(), dbus::Error> {
    let is_player = |name: &str| name.starts_with("org.mpris.MediaPlayer2.");
    let bus = Connection::new_session()?;
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
        bus.channel()
            .read_write(None)
            .map_err(|()| dbus::Error::new_failed("MPRIS session bus disconnected"))?;
    }
}
