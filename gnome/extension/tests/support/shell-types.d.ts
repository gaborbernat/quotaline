import 'resource:///org/gnome/shell/misc/extensionUtils.js';
import 'resource:///org/gnome/shell/ui/panel.js';
import type St from 'gi://St';

// The published Shell typings still use the names from before GNOME 46.
declare module 'resource:///org/gnome/shell/misc/extensionUtils.js' {
    enum ExtensionState {
        ACTIVE = 1,
        INACTIVE = 2,
        DEACTIVATING = 7,
        ACTIVATING = 8,
    }
}

declare module 'resource:///org/gnome/shell/ui/panel.js' {
    interface Panel {
        _centerBox: St.BoxLayout;
    }
}
