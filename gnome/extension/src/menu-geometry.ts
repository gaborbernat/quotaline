export function menuWidth(
    indicatorX: number,
    indicatorWidth: number,
    monitor: {x: number; width: number},
    maximum: number,
): number {
    const center = indicatorX + indicatorWidth / 2;
    const centered = Math.max(0, Math.min(center - monitor.x - 16, monitor.x + monitor.width - center - 16)) * 2;
    return Math.min(monitor.width - 32, Math.max(620, Math.min(maximum, centered)));
}
