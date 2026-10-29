//! A set of actuator IDs, as a bitset (research R-16).

use crate::Id;

/// A set of actuator [`Id`]s: a 32-byte bitset, `Copy`, iterated in
/// ascending order.
///
/// Any `IntoIterator<Item = Id>` names the targets of a multi-actuator call;
/// an `IdSet` is one that owns its IDs, so it can be built from the bus it is
/// passed to:
///
/// ```
/// use dxl_rs_v2::{Id, IdSet};
///
/// let mut ids = IdSet::range(Id::new(1)?, Id::new(3)?);
/// assert!(!ids.insert(Id::new(2)?));
/// assert!(ids.insert(Id::new(9)?));
/// let order: Vec<u8> = ids.into_iter().map(Id::get).collect();
/// assert_eq!(order, [1, 2, 3, 9]);
/// # Ok::<(), dxl_rs_v2::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IdSet {
    /// Bit `n` set ⇔ `Id(n)` present. Only bits 0..=252 are used.
    bits: [u32; 8],
}

/// The word and the bit mask of raw ID `n`. `n >> 5` is at most 7, so the
/// word index is always in range.
const fn slot(n: u8) -> (usize, u32) {
    ((n >> 5) as usize, 1 << (n & 31))
}

impl IdSet {
    /// The empty set.
    pub const fn new() -> Self {
        Self { bits: [0; 8] }
    }

    /// The IDs `first..=last`; empty if `first > last`.
    pub const fn range(first: Id, last: Id) -> Self {
        let mut set = Self::new();
        let mut n = first.get();
        while n <= last.get() {
            let (word, mask) = slot(n);
            set.bits[word] |= mask;
            n += 1;
        }
        set
    }

    /// Adds `id`. Returns `false` if already present.
    pub fn insert(&mut self, id: Id) -> bool {
        let (word, mask) = slot(id.get());
        let present = self.bits[word] & mask != 0;
        self.bits[word] |= mask;
        !present
    }

    /// Removes `id`. Returns `false` if it was not present.
    pub fn remove(&mut self, id: Id) -> bool {
        let (word, mask) = slot(id.get());
        let present = self.bits[word] & mask != 0;
        self.bits[word] &= !mask;
        present
    }

    /// Whether `id` is in the set.
    pub const fn contains(&self, id: Id) -> bool {
        let (word, mask) = slot(id.get());
        self.bits[word] & mask != 0
    }

    /// The number of IDs in the set.
    pub const fn len(&self) -> usize {
        let mut count = 0;
        let mut word = 0;
        while word < self.bits.len() {
            count += self.bits[word].count_ones() as usize;
            word += 1;
        }
        count
    }

    /// Whether the set holds no ID.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The IDs of an [`IdSet`], ascending.
#[derive(Debug, Clone)]
pub struct IdSetIter {
    /// The IDs not yet returned.
    bits: [u32; 8],
}

impl Iterator for IdSetIter {
    type Item = Id;

    fn next(&mut self) -> Option<Id> {
        for (word, bits) in (0u8..).zip(self.bits.iter_mut()) {
            if *bits != 0 {
                let bit = bits.trailing_zeros();
                *bits &= *bits - 1;
                return u8::try_from(bit)
                    .ok()
                    .and_then(|bit| Id::new(word * 32 + bit).ok());
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = IdSet { bits: self.bits }.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for IdSetIter {}

impl IntoIterator for IdSet {
    type Item = Id;
    type IntoIter = IdSetIter;

    /// The IDs in ascending order.
    fn into_iter(self) -> IdSetIter {
        IdSetIter { bits: self.bits }
    }
}

impl FromIterator<Id> for IdSet {
    /// The set of every ID `ids` yields; repeats are kept once.
    fn from_iter<I: IntoIterator<Item = Id>>(ids: I) -> Self {
        let mut set = Self::new();
        for id in ids {
            set.insert(id);
        }
        set
    }
}

impl Extend<Id> for IdSet {
    /// Adds every ID `ids` yields.
    fn extend<I: IntoIterator<Item = Id>>(&mut self, ids: I) {
        for id in ids {
            self.insert(id);
        }
    }
}

impl<const K: usize> From<[Id; K]> for IdSet {
    /// The set of the IDs in `ids`; repeats are kept once.
    fn from(ids: [Id; K]) -> Self {
        ids.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use super::*;
    use crate::bus::testing::id;

    fn raw(set: IdSet) -> Vec<u8> {
        set.into_iter().map(Id::get).collect()
    }

    #[test]
    fn new_and_default_are_empty() {
        assert_eq!(IdSet::new(), IdSet::default());
        assert!(IdSet::new().is_empty());
        assert_eq!(IdSet::new().len(), 0);
        assert_eq!(raw(IdSet::new()), [] as [u8; 0]);
    }

    #[test]
    fn range_holds_first_to_last() {
        let set = IdSet::range(id(2), id(5));
        assert_eq!(raw(set), [2, 3, 4, 5]);
        assert_eq!(set.len(), 4);
        assert!(IdSet::range(id(5), id(2)).is_empty());
        assert_eq!(IdSet::range(id(7), id(7)).len(), 1);
        let all = IdSet::range(id(0), id(252));
        assert_eq!(all.len(), 253);
        assert_eq!(raw(all), (0..=252).collect::<Vec<u8>>());
    }

    #[test]
    fn insert_remove_contains() {
        let mut set = IdSet::new();
        assert!(set.insert(id(40)));
        assert!(!set.insert(id(40)));
        assert!(set.contains(id(40)));
        assert!(!set.contains(id(41)));
        assert!(set.remove(id(40)));
        assert!(!set.remove(id(40)));
        assert!(!set.contains(id(40)));
        assert!(set.is_empty());
    }

    #[test]
    fn iterates_ascending_whatever_the_insert_order() {
        let mut set = IdSet::new();
        for n in [252, 3, 64, 0, 31, 32, 100] {
            set.insert(id(n));
        }
        assert_eq!(raw(set), [0, 3, 31, 32, 64, 100, 252]);
        let iter = set.into_iter();
        assert_eq!(iter.len(), 7);
        let again = iter.clone();
        assert_eq!(iter.count(), again.count());
    }

    #[test]
    fn from_iterator_array_and_extend() {
        let collected: IdSet = [id(9), id(1), id(9)].into_iter().collect();
        assert_eq!(raw(collected), [1, 9]);
        let from = IdSet::from([id(4), id(2)]);
        assert_eq!(raw(from), [2, 4]);
        let mut extended = from;
        extended.extend([id(3)]);
        assert_eq!(raw(extended), [2, 3, 4]);
    }

    #[test]
    fn is_copy_and_32_bytes() {
        let set = IdSet::from([id(1)]);
        let moved = set;
        assert!(set.contains(id(1)) && moved.contains(id(1)));
        assert_eq!(size_of::<IdSet>(), 32);
    }
}
