// Words durations for live Shell rendering. Rust words the same durations for notifications and macOS, and both
// test suites read crates/presentation/text/tests/fixtures/time-text.json so the wording cannot drift.

export function countdown(seconds: number): string {
    if (seconds <= 0) return 'reset due';
    const minutes = Math.trunc(seconds / 60);
    const days = Math.trunc(minutes / 1440);
    const hours = Math.trunc((minutes % 1440) / 60);
    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours} hr ${minutes % 60} min`;
    return minutes > 0 ? `${minutes} min` : '< 1 min';
}

export function compactCountdown(seconds: number): string {
    if (seconds <= 0) return '0m';
    const minutes = Math.trunc(seconds / 60);
    const days = Math.trunc(minutes / 1440);
    const hours = Math.trunc((minutes % 1440) / 60);
    if (days > 0) return `${days}d${hours}h`;
    if (hours > 0) return `${hours}h${String(minutes % 60).padStart(2, '0')}m`;
    return `${minutes}m`;
}

export function relativeAge(seconds: number): string {
    seconds = Math.max(0, seconds);
    if (seconds < 5) return 'just now';
    if (seconds < 60) return `${Math.trunc(seconds)}s ago`;
    if (seconds < 3600) return `${Math.trunc(seconds / 60)} min ago`;
    if (seconds < 86_400) return `${Math.trunc(seconds / 3600)} hr ago`;
    return `${Math.trunc(seconds / 86_400)} d ago`;
}
