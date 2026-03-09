import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const DBUS_NAME = 'com.github.voskdictation';
const DBUS_PATH = '/com/github/voskdictation';
const DBUS_IFACE = 'com.github.voskdictation';
const DAEMON_BIN = '/usr/local/bin/vosk-dictation';

const DaemonInterface = `
<node>
  <interface name="${DBUS_IFACE}">
    <method name="Enable"/>
    <method name="Disable"/>
    <method name="Quit"/>
    <property name="State" type="s" access="read"/>
    <signal name="StatusChanged">
      <arg type="s" name="state"/>
    </signal>
  </interface>
</node>
`;

const DaemonProxy = Gio.DBusProxy.makeProxyWrapper(DaemonInterface);

export default class VoskDictationExtension extends Extension {
    enable() {
        this._daemonRunning = false;
        this._togglingInternally = false;

        this._indicator = new PanelMenu.Button(0.0, this.metadata.name, false);

        // Panel icon
        this._icon = new St.Icon({
            icon_name: 'microphone-disabled-symbolic',
            style_class: 'system-status-icon',
        });
        this._indicator.add_child(this._icon);

        this._buildMenu();
        Main.panel.addToStatusArea(this.uuid, this._indicator);

        // Watch for the daemon appearing/disappearing on the bus
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            DBUS_NAME,
            Gio.BusNameWatcherFlags.NONE,
            () => this._onDaemonAppeared(),
            () => this._onDaemonVanished(),
        );

        this._updateUI('disconnected');
    }

    disable() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = null;
        }
        if (this._pollId) {
            GLib.source_remove(this._pollId);
            this._pollId = null;
        }
        this._disconnectProxy();
        this._indicator?.destroy();
        this._indicator = null;
        this._icon = null;
        this._toggleItem = null;
        this._statusLabel = null;
    }

    _buildMenu() {
        // Enable/Disable toggle
        this._toggleItem = new PopupMenu.PopupSwitchMenuItem('Enable Dictation', false);
        this._toggleItem.connect('toggled', (_item, active) => {
            if (this._togglingInternally)
                return;

            if (active) {
                if (!this._daemonRunning)
                    this._startDaemon();
                else
                    this._callDaemon('Enable');
            } else {
                this._callDaemon('Disable');
            }
        });
        this._indicator.menu.addMenuItem(this._toggleItem);

        this._indicator.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // Status label
        this._statusLabel = new PopupMenu.PopupMenuItem('Status: Daemon not running', {reactive: false});
        this._indicator.menu.addMenuItem(this._statusLabel);

        this._indicator.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

        // Quit button
        this._indicator.menu.addAction('Quit Daemon', () => {
            this._callDaemon('Quit');
        });
    }

    _onDaemonAppeared() {
        this._daemonRunning = true;
        this._connectProxy();
    }

    _onDaemonVanished() {
        this._daemonRunning = false;
        this._disconnectProxy();
        this._updateUI('disconnected');
    }

    _connectProxy() {
        try {
            this._proxy = new DaemonProxy(
                Gio.DBus.session,
                DBUS_NAME,
                DBUS_PATH,
            );

            this._signalId = this._proxy.connectSignal(
                'StatusChanged',
                (_proxy, _sender, [state]) => {
                    this._updateUI(state);
                },
            );

            this._propId = this._proxy.connect(
                'g-properties-changed',
                () => this._refreshState(),
            );

            // Poll state periodically since property change signals
            // may not fire for every transition
            this._pollId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
                this._refreshState();
                return GLib.SOURCE_CONTINUE;
            });

            this._refreshState();
        } catch (e) {
            console.error(`VoskDictation: Failed to connect proxy: ${e.message}`);
            this._updateUI('disconnected');
        }
    }

    _disconnectProxy() {
        if (this._pollId) {
            GLib.source_remove(this._pollId);
            this._pollId = null;
        }
        if (this._proxy) {
            if (this._signalId !== undefined)
                this._proxy.disconnectSignal(this._signalId);
            if (this._propId !== undefined)
                this._proxy.disconnect(this._propId);
        }
        this._proxy = null;
        this._signalId = undefined;
        this._propId = undefined;
    }

    _refreshState() {
        if (!this._proxy)
            return;

        try {
            const state = this._proxy.State;
            if (state)
                this._updateUI(state);
        } catch (e) {
            this._updateUI('disconnected');
        }
    }

    _updateUI(state) {
        if (!this._indicator)
            return;

        // Update status label
        if (this._statusLabel) {
            const labels = {
                'idle': 'Status: Idle (waiting for text field)',
                'listening': 'Status: Listening...',
                'paused': 'Status: Paused',
                'disconnected': 'Status: Daemon not running',
            };
            this._statusLabel.label.text = labels[state] || `Status: ${state}`;
        }

        // Update toggle switch without triggering the callback
        if (this._toggleItem) {
            const isActive = state !== 'paused' && state !== 'disconnected';
            if (this._toggleItem.state !== isActive) {
                this._togglingInternally = true;
                this._toggleItem.setToggleState(isActive);
                this._togglingInternally = false;
            }
        }

        // Update panel icon
        if (this._icon) {
            const iconName = (state === 'paused' || state === 'disconnected')
                ? 'microphone-disabled-symbolic'
                : 'audio-input-microphone-symbolic';
            this._icon.icon_name = iconName;
        }
    }

    _startDaemon() {
        try {
            const [ok] = GLib.spawn_command_line_async(DAEMON_BIN);
            if (ok) {
                this._updateUI('idle');
                // Daemon will appear on the bus shortly, _onDaemonAppeared will handle it
            }
        } catch (e) {
            console.error(`VoskDictation: Failed to start daemon: ${e.message}`);
            Main.notifyError(
                'VoskDictation',
                `Could not start daemon: ${e.message}`,
            );
            this._updateUI('disconnected');
        }
    }

    _callDaemon(method) {
        if (!this._proxy) {
            if (method === 'Enable')
                this._startDaemon();
            else
                console.error('VoskDictation: Daemon not running');
            return;
        }

        this._proxy[`${method}Remote`]().catch(e => {
            console.error(`VoskDictation: D-Bus ${method} failed: ${e.message}`);
            if (method !== 'Quit')
                this._updateUI('disconnected');
        });
    }
}
