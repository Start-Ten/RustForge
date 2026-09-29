//! 事件双缓冲（IF-076）。

use crate::Resource;
use rf_core::Tick;

/// 类型化事件存储（双缓冲：当前帧 + 上一帧）。
pub struct Events<E> {
    current: Vec<E>,
    previous: Vec<E>,
    updated_at: Tick,
}

impl<E: Send + Sync + 'static> Resource for Events<E> {}

impl<E> Default for Events<E> {
    fn default() -> Self {
        Self { current: Vec::new(), previous: Vec::new(), updated_at: Tick(0) }
    }
}

impl<E> Events<E> {
    pub fn send(&mut self, event: E) {
        self.current.push(event);
    }

    /// 帧末调用：current → previous，current 清空。保留 2 帧。
    pub fn update(&mut self, tick: Tick) {
        self.updated_at = tick;
        self.previous = std::mem::take(&mut self.current);
    }

    pub fn len_total(&self) -> usize {
        self.current.len() + self.previous.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len_total() == 0
    }

    pub fn current(&self) -> &[E] {
        &self.current
    }

    pub fn previous(&self) -> &[E] {
        &self.previous
    }

    pub fn updated_at(&self) -> Tick {
        self.updated_at
    }

    /// 从全局游标读取（供 EventReader 使用）。
    pub(crate) fn get(&self, index: usize) -> Option<&E> {
        if index < self.previous.len() {
            self.previous.get(index)
        } else {
            self.current.get(index - self.previous.len())
        }
    }

    pub(crate) fn total(&self) -> usize {
        self.len_total()
    }
}

/// 事件读取器（IF-076）：游标式，跨双缓冲连续读取。
#[derive(Debug, Clone, Default)]
pub struct EventReader {
    cursor: usize,
}

impl EventReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// 读取自上次读取以来的全部事件并推进游标。
    pub fn iter<'a, E>(&mut self, events: &'a Events<E>) -> impl Iterator<Item = &'a E> {
        let start = self.cursor.min(events.total());
        self.cursor = events.total();
        (start..events.total()).filter_map(move |i| events.get(i))
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_buffer_lifecycle() {
        let mut ev = Events::<u32>::default();
        let mut reader = EventReader::new();
        ev.send(1);
        ev.send(2);
        let read: Vec<u32> = reader.iter(&ev).copied().collect();
        assert_eq!(read, vec![1, 2]);
        ev.update(Tick(1));
        assert!(reader.iter(&ev).next().is_none()); // 已读
        ev.send(3);
        let read: Vec<u32> = reader.iter(&ev).copied().collect();
        assert_eq!(read, vec![3]);
        ev.update(Tick(2));
        ev.update(Tick(3));
        // 两帧后 [1,2,3] 全部离场
        assert!(ev.is_empty());
    }
}
