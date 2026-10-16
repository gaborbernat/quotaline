import type GObject from 'gi://GObject';
import type * as Signals from '@girs/gnome-shell/misc/signals';
import type {Calendar} from '@girs/gnome-shell/ui/calendar';

declare module 'resource:///org/gnome/shell/ui/calendar.js' {
    // Shell exports this base, whose members throw until overridden, but the @girs typings keep it module-private.
    type EventSource = Parameters<Calendar['setEventSource']>[0];

    export class EventSourceBase extends GObject.Object {
        readonly isLoading: boolean;
        readonly hasCalendars: boolean;
        destroy(): void;
        requestRange(begin: Date, end: Date): void;
        getEvents(begin: Date, end: Date): ReturnType<EventSource['getEvents']>;
        hasEvents(day: Date): boolean;
    }
}

declare module '@girs/gnome-shell/ui/popupMenu' {
    // Shell tests the animation flags bitwise, so false opens and closes without animating, as Shell's own callers do.
    interface PopupMenu<S extends Signals.SignalMap<S> = PopupMenu.SignalMap> {
        open(animate: boolean): void;
        close(animate: boolean): void;
    }
}
