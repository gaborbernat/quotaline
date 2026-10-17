import type {PaceStatus} from './service-state.js';

interface Tone {
    hue: number;
    saturation: number;
    brightness: number;
}

interface Palette {
    green: Tone;
    orange: Tone;
    red: Tone;
}

const usagePalette = {
    bar: {
        green: {hue: 0.38, saturation: 0.72, brightness: 0.52},
        orange: {hue: 0.08, saturation: 0.78, brightness: 1},
        red: {hue: 0.025, saturation: 0.55, brightness: 1},
    },
    dark_text: {
        green: {hue: 0.38, saturation: 0.7, brightness: 1},
        orange: {hue: 0.08, saturation: 0.64, brightness: 1},
        red: {hue: 0.025, saturation: 0.38, brightness: 1},
    },
    status_text: {
        green: {hue: 0.38, saturation: 0.7, brightness: 1},
        orange: {hue: 0.08, saturation: 0.78, brightness: 1},
        red: {hue: 0.025, saturation: 0.55, brightness: 1},
    },
    light_text: {
        green: {hue: 0.401, saturation: 0.671, brightness: 0.24},
        orange: {hue: 0.063, saturation: 0.676, brightness: 0.336},
        red: {hue: 0.026, saturation: 0.654, brightness: 0.353},
    },
    orange_at: 0.6,
};

export function usageColor(percent: number, pace: PaceStatus | null = null): string {
    return colorFor(percent, pace, usagePalette.bar);
}

export function usageTextColor(percent: number, pace: PaceStatus | null = null, light = false): string {
    return colorFor(percent, pace, light ? usagePalette.light_text : usagePalette.dark_text);
}

export function usageStatusColor(percent: number, pace: PaceStatus | null = null, light = false): string {
    return colorFor(percent, pace, light ? usagePalette.light_text : usagePalette.status_text);
}

function colorFor(percent: number, pace: PaceStatus | null, {green, orange, red}: Palette): string {
    let hue: number;
    let saturation: number;
    let brightness: number;
    if (pace === 'ahead') ({hue, saturation, brightness} = orange);
    else if (pace === 'exhausted') ({hue, saturation, brightness} = red);
    else {
        const fraction = Math.min(100, Math.max(0, percent)) / 100;
        const boundary = usagePalette.orange_at;
        const [from, to, amount] =
            fraction < boundary
                ? [green, orange, (fraction / boundary) ** 2]
                : [orange, red, (fraction - boundary) / (1 - boundary)];
        hue = from.hue + (to.hue - from.hue) * amount;
        saturation = from.saturation + (to.saturation - from.saturation) * amount;
        brightness = from.brightness + (to.brightness - from.brightness) * amount;
    }
    const chroma = brightness * saturation;
    const sector = hue * 6;
    const secondary = chroma * (1 - Math.abs((sector % 2) - 1));
    const base = brightness - chroma;
    const channels = sector < 1 ? [chroma, secondary, 0] : sector < 2 ? [secondary, chroma, 0] : [0, chroma, secondary];
    return `#${channels
        .map(value =>
            Math.round((value + base) * 255)
                .toString(16)
                .padStart(2, '0'),
        )
        .join('')}`;
}
