pub struct LogEntry<T: Default + Copy> {
    term: u32,
    entry: T,
}
