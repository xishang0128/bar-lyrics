import QtQuick
import qs.Modules.Plugins

PluginSettings {
    pluginId: "barLyrics"

    StringSetting {
        settingKey: "base_url"
        label: "SPlayer API"
        defaultValue: "http://127.0.0.1:14558"
    }

    SelectionSetting {
        settingKey: "subtitle"
        label: "副行 / Subtitle"
        defaultValue: "auto"
        options: [
            {
                "label": "自动 / Auto",
                "value": "auto"
            },
            {
                "label": "翻译 / Translation",
                "value": "translation"
            },
            {
                "label": "音译 / Romanization",
                "value": "romanization"
            },
            {
                "label": "隐藏 / Hidden",
                "value": "hidden"
            }
        ]
    }

    SelectionSetting {
        settingKey: "alignment"
        label: "歌词对齐 / Alignment"
        defaultValue: "end"
        options: [
            {
                "label": "左 / Left",
                "value": "start"
            },
            {
                "label": "右 / Right",
                "value": "end"
            }
        ]
    }

    SelectionSetting {
        settingKey: "cover_position"
        label: "封面位置 / Cover position"
        defaultValue: "right"
        options: [
            {
                "label": "左 / Left",
                "value": "left"
            },
            {
                "label": "右 / Right",
                "value": "right"
            }
        ]
    }

    ToggleSetting {
        settingKey: "show_cover"
        label: "显示封面 / Cover"
        defaultValue: true
    }

    ToggleSetting {
        settingKey: "show_when_paused"
        label: "暂停时显示 / Show when paused"
        defaultValue: true
    }

    SliderSetting {
        settingKey: "max_width"
        label: "最大宽度 / Maximum width"
        defaultValue: 360
        minimum: 80
        maximum: 800
        unit: "px"
    }

    SliderSetting {
        settingKey: "max_chars"
        label: "最大字符数 / Maximum characters"
        defaultValue: 32
        minimum: 12
        maximum: 80
    }

    SliderSetting {
        settingKey: "single_line_font_size"
        label: "单行字号 / Single line"
        defaultValue: 15
        minimum: 8
        maximum: 32
        unit: "px"
    }

    SliderSetting {
        settingKey: "multiline_main_font_size"
        label: "多行主行字号 / Main line"
        defaultValue: 13
        minimum: 8
        maximum: 32
        unit: "px"
    }

    SliderSetting {
        settingKey: "multiline_subtitle_font_size"
        label: "副行字号 / Subtitle"
        defaultValue: 9
        minimum: 8
        maximum: 24
        unit: "px"
    }

    SliderSetting {
        settingKey: "inactive_opacity"
        label: "未唱部分不透明度 / Inactive opacity"
        defaultValue: 45
        minimum: 0
        maximum: 100
        unit: "%"
    }

    SliderSetting {
        settingKey: "offset_ms"
        label: "歌词偏移 / Lyric offset"
        defaultValue: 0
        minimum: -10000
        maximum: 10000
        unit: "ms"
    }
}
