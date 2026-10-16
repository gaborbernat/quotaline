import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {TokenPanel} from './src/panel.js';

export default class QuotalineExtension extends Extension {
    _sessionQuit: boolean;
    _panel?: TokenPanel | null;

    constructor(metadata: ConstructorParameters<typeof Extension>[0]) {
        super(metadata);
        this._sessionQuit = false;
    }

    override enable(): void {
        if (this._sessionQuit) return;
        const {url} = this.metadata;
        this._panel = new TokenPanel(this.uuid, this.path, url as string, () => {
            this._quit();
        });
        this._panel.enable();
    }

    _quit(): void {
        this._sessionQuit = true;
        this._panel?.disable();
        this._panel = null;
    }

    override disable(): void {
        this._panel?.disable();
        this._panel = null;
        if (!global.settings.get_strv('enabled-extensions').includes(this.uuid)) this._sessionQuit = false;
    }
}
