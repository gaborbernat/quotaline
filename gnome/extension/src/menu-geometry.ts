export function menuWidth(monitorWidth: number, maximum: number): number {
    return Math.min(monitorWidth - 32, maximum);
}
