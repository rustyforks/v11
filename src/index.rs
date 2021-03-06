//! `GenericRowId`, `CheckedRowId`, and `RowRange`.
// FIXME: Need https://github.com/rust-lang/rust/issues/38078 to ditch obnoxious verbosity
// FIXME: s/GenericRowId/FutureCheckRowId? s/UncheckedRowId/PastCheckRowId ?
// FIXME: This should be several sub-modules.

use std::fmt;
use std::marker::PhantomData;
use std::cmp::{Ordering, Eq, PartialEq, PartialOrd, Ord};
use std::sync::RwLock;
use std::cell::Cell;
use std::ops::Deref;

use num_traits::{ToPrimitive, One, Bounded};
use num_traits::cast::FromPrimitive;

use crate::Universe;
use crate::tables::{GetTableName, LockedTable, GenericTable};


/// Index to a row on some table.
/// You can call `row_index.check(&table)` to pre-check the index,
/// which you should do if you will be accessing multiple columns.
pub struct GenericRowId<T: GetTableName> {
    #[doc(hidden)]
    pub i: T::Idx,
    #[doc(hidden)]
    pub table: PhantomData<T>,
}
use serde::ser::{Serialize, Serializer};
impl<T: GetTableName> Serialize for GenericRowId<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        self.i.serialize(serializer)
    }
}
use serde::de::{Deserialize, Deserializer};
impl<'de, T: GetTableName> Deserialize<'de> for GenericRowId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let i = T::Idx::deserialize(deserializer)?;
        Ok(GenericRowId::new(i))
    }
}
impl<T: GetTableName> GenericRowId<T> where Self: ::serde::Serialize {
    pub fn new(i: T::Idx) -> Self {
        GenericRowId {
            i,
            table: PhantomData,
        }
    }

    pub fn from_usize(i: usize) -> Self where T::Idx: FromPrimitive {
        Self::new(T::Idx::from_usize(i).unwrap())
    }
    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> T::Idx { self.i }

    pub fn next(&self) -> Self {
        Self::new(self.i + T::Idx::one())
    }

    pub fn prev(&self) -> Self {
        Self::new(self.i - T::Idx::one())
    }

    pub fn get_domain() -> crate::domain::DomainName { T::get_domain() }
    pub fn get_name() -> crate::tables::TableName { T::get_name() }
    pub fn get_generic_table(universe: &Universe) -> &RwLock<GenericTable> {
        let domain_id = Self::get_domain().get_id();
        universe.get_generic_table(domain_id, Self::get_name())
    }
}


/// This value can be used to index into table columns.
/// It borrows the table to ensure that it is a valid index.
#[derive(Hash)]
pub struct CheckedRowId<'a, T: LockedTable + 'a> {
    i: <T::Row as GetTableName>::Idx,
    // FIXME: This should be a PhantomData. NBD since these things are short-lived.
    table: &'a T,
}
impl<'a, T: LockedTable + 'a> CheckedRowId<'a, T> {
    /// Create a `CheckedRowId` without actually checking.
    pub unsafe fn fab(i: <T::Row as GetTableName>::Idx, table: &'a T) -> Self {
        Self { i, table }
    }
    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> <T::Row as GetTableName>::Idx { self.i }
    pub fn next(self) -> GenericRowId<T::Row> { self.uncheck().next() }
}





// Easy, right? WRONG!
// We `#[derive]`d nothing! f$cking phantom data!


impl<T: GetTableName> Default for GenericRowId<T> {
    fn default() -> Self {
        GenericRowId {
            i: T::Idx::max_value() /* UNDEFINED_INDEX */,
            table: PhantomData,
        }
    }
}
// `Checked: Default` is unsound.


impl<T: GetTableName> Clone for GenericRowId<T> {
    fn clone(&self) -> Self {
        Self { i: self.i, table: self.table }
    }
}
impl<'a, T: LockedTable + 'a> Clone for CheckedRowId<'a, T> where <T::Row as GetTableName>::Idx: Copy {
    fn clone(&self) -> Self {
        Self { i: self.i, table: self.table }
    }
}


impl<T: GetTableName> Copy for GenericRowId<T> {}
impl<'a, T: LockedTable + 'a> Copy for CheckedRowId<'a, T> where <T::Row as GetTableName>::Idx: Copy {}


impl<T: GetTableName> fmt::Debug for GenericRowId<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::get_name().0, self.i)
    }
}
impl<'a, T: LockedTable + 'a> fmt::Debug for CheckedRowId<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::Row::get_name().0, self.i)
    }
}

macro_rules! dispatch {
    ($($Trait:ident => $fn:ident -> $ret:ty,)*) => {$(
        impl<T: GetTableName> $Trait for GenericRowId<T> {
            fn $fn(&self, other: &Self) -> $ret {
                self.i.$fn(&other.i)
            }
        }
        impl<'a, T: LockedTable + 'a> $Trait for CheckedRowId<'a, T> {
            fn $fn(&self, other: &Self) -> $ret {
                self.i.$fn(&other.i)
            }
        }
    )*};
}
dispatch! {
    PartialEq => eq -> bool,
    PartialOrd => partial_cmp -> Option<Ordering>,
    Ord => cmp -> Ordering,
}

impl<T: GetTableName> Eq for GenericRowId<T> {}
impl<'a, T: LockedTable + 'a> Eq for CheckedRowId<'a, T> {}


use std::hash::{Hash, Hasher};
impl<T: GetTableName> Hash for GenericRowId<T>
where T::Idx: Hash
{
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.i.hash(state);
    }
}





// We've escaped that particular hell.
// Now we implement comparisons between Checked & Unchecked.
macro_rules! cmp {
    ($a:ty, $b:ty) => {
        cmp!(impl, $a, $b);
        cmp!(impl, $b, $a);
    };
    (impl, $left:ty, $right:ty) => {
        // $right goes on the $left, natch.
        // (It's actually not too bad, since it's got the `Rhs` label)
        impl<'a, T: LockedTable + 'a> PartialEq<$right> for $left {
            fn eq(&self, rhs: &$right) -> bool {
                self.i == rhs.i
            }
        }
        impl<'a, T: LockedTable + 'a> PartialOrd<$right> for $left {
            fn partial_cmp(&self, rhs: &$right) -> Option<Ordering> {
                Some(self.i.cmp(&rhs.i))
            }
        }
    };
}
cmp!(CheckedRowId<'a, T>, GenericRowId<T::Row>);



#[cfg(test)]
mod test {
    use super::*;
    use crate::tables::{TableName, LockedTable, Guarantee};
    use crate::domain::DomainName;

    struct TestName;
    impl GetTableName for TestName {
        type Idx = usize;
        fn get_domain() -> DomainName { DomainName("test_domain") }
        fn get_name() -> TableName { TableName("test_table") }
        fn get_guarantee() -> Guarantee { Guarantee { consistent: false, sorted: false, append_only: false } }
        fn get_generic_table(_: &Universe) -> &::std::sync::RwLock<GenericTable> { unimplemented!() }
        fn new_generic_table() -> GenericTable { unimplemented!() }
    }
    struct TestTable;
    impl LockedTable for TestTable {
        type Row = TestName;
        fn len(&self) -> usize { 14 }
    }

    #[test]
    fn test_formatting() {
        let gen: GenericRowId<TestName> = GenericRowId {
            i: 23,
            table: ::std::marker::PhantomData,
        };
        assert_eq!("test_table[23]", format!("{:?}", gen));
    }

    #[test]
    fn eq() {
        let my_table = TestTable;
        let checked = CheckedRowId {
            i: 10,
            table: &my_table,
        };
        let unchecked = GenericRowId {
            i: 10,
            table: PhantomData,
        };
        assert_eq!(checked, unchecked);
        assert_eq!(unchecked, checked);
    }

    #[test]
    fn cmp() {
        let my_table = TestTable;
        let checked = CheckedRowId {
            i: 3,
            table: &my_table,
        };
        let unchecked = GenericRowId {
            i: 10,
            table: PhantomData,
        };
        assert!(checked < unchecked);
        assert!(unchecked >= checked);
    }
}






#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[derive(Serialize, Deserialize)]
pub struct RowRange<R> {
    pub start: R,
    pub end: R,
}
use std::ops::Range;
impl<R> Into<Range<R>> for RowRange<R> {
    fn into(self) -> Range<R> {
        self.start..self.end
    }
}
impl<R> From<Range<R>> for RowRange<R> {
    fn from(range: Range<R>) -> RowRange<R> {
        RowRange {
            start: range.start,
            end: range.end,
        }
    }
}
impl<T: GetTableName> RowRange<GenericRowId<T>> {
    #[inline]
    pub fn empty() -> Self {
        RowRange {
            start: GenericRowId::default(),
            end: GenericRowId::default(),
        }
    }

    /// Create a `RowRange` over a single element.
    #[inline]
    pub fn on(i: GenericRowId<T>) -> Self {
        RowRange {
            start: i,
            end: i.next(),
        }
    }

    /// Return the `n`th row after the start, if it is within the range.
    #[inline]
    pub fn offset(&self, n: T::Idx) -> Option<GenericRowId<T>> {
        use num_traits::CheckedAdd;
        let at = self.start.to_raw().checked_add(&n);
        let at = if let Some(at) = at {
            at
        } else {
            return None;
        };
        if at > self.end.to_raw() {
            None
        } else {
            Some(GenericRowId::new(at))
        }
    }

    /// Return how many rows are in this range.
    #[inline]
    pub fn len(&self) -> usize {
        self.end.to_usize() - self.start.to_usize()
    }

    /// Return `true` if the given row is within this range.
    #[inline]
    pub fn contains(&self, o: GenericRowId<T>) -> bool {
        self.start <= o && o < self.end
    }

    /// Return `true` if this range overlaps with another.
    #[inline]
    pub fn intersects(&self, other: Self) -> bool {
        // If we're not intersecting, then one is to the right of the other.
        // <-- (a.0, a.1) -- (b.0, b.1) -->
        // <-- (b.0, b.1) -- (a.0, a.1) -->
        debug_assert!(self.start <= self.end);
        debug_assert!(other.start <= other.end);
        // This usual pattern is for inclusive ranges, but this is an exclusive range.
        !(self.end <= other.start || other.end <= self.start)
    }

    /// If the given row is within this RowRange, return its offset from the beginning.
    #[inline]
    pub fn inner_index(&self, o: GenericRowId<T>) -> Option<T::Idx> {
        if self.contains(o) {
            Some(o.to_raw() - self.start.to_raw())
        } else {
            None
        }
    }

    #[inline]
    pub fn iter_slow(&self) -> UncheckedIter<T> {
        UncheckedIter {
            i: self.start.to_raw(),
            end: self.end.to_raw(),
        }
    }
}

#[cfg(test)]
mod row_range_test {
    use super::*;
    use crate::tables::{TableName, Guarantee};
    use crate::domain::DomainName;
    struct TestTable;
    impl GetTableName for TestTable {
        type Idx = usize;
        fn get_domain() -> DomainName { DomainName("TEST_DOMAIN") }
        fn get_name() -> TableName { TableName("test_table") }
        fn get_guarantee() -> Guarantee { Guarantee { consistent: false, sorted: false, append_only: false } }
        fn get_generic_table(_: &Universe) -> &::std::sync::RwLock<GenericTable> { unimplemented!() }
        fn new_generic_table() -> GenericTable { unimplemented!() }
    }
    type RR = RowRange<GenericRowId<TestTable>>;

    fn new(start: usize, end: usize) -> RR {
        RR {
            start: GenericRowId::new(start),
            end: GenericRowId::new(end),
        }
    }

    #[test]
    fn intersection() {
        let right = new(8, 9);
        let left = new(0, 8);
        assert!(right.intersects(right));
        assert!(left.intersects(left));
        assert!(!right.intersects(left));
        assert!(!left.intersects(right));
        let mid = new(3, 5);
        assert!(left.intersects(mid));
        assert!(mid.intersects(left));
        assert!(!right.intersects(mid));
        assert!(!mid.intersects(right));
    }
}


#[derive(Debug, Clone)]
pub struct CheckedIter<'a, T: LockedTable + 'a> {
    table: &'a T,
    i: <T::Row as GetTableName>::Idx,
    end: <T::Row as GetTableName>::Idx,
}
impl<'a, T: LockedTable> CheckedIter<'a, T> {
    pub fn from(table: &'a T, slice: RowRange<GenericRowId<T::Row>>) -> Self {
        assert!(slice.start.to_usize() < table.len() || (slice.start == slice.end));
        assert!(slice.end.to_usize() <= table.len()); // Remember: end is excluded from the iteration!
        CheckedIter {
            table,
            i: slice.start.to_raw(),
            end: slice.end.to_raw(),
        }
    }

    /// Convert to an iterator yielding `GenericRowId`s instead of `CheckedRowId`s.
    pub fn uncheck(self) -> RowRange<GenericRowId<T::Row>> {
        RowRange {
            start: GenericRowId::new(self.i),
            end: GenericRowId::new(self.end),
        }
    }

    pub fn skip_fast(mut self, n: usize) -> Self {
        self.i = (<T::Row as GetTableName>::Idx::from_usize(self.i.to_usize().unwrap() + n)).unwrap();
        self
    }
}
impl<'a, T: LockedTable> Iterator for CheckedIter<'a, T> {
    type Item = CheckedRowId<'a, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end {
            None
        } else {
            let ret = CheckedRowId {
                i: self.i,
                table: self.table,
            };
            self.i = ret.next().i;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let s = self.end.to_usize().unwrap() - self.i.to_usize().unwrap();
        (s, Some(s))
    }
}

pub struct UncheckedIter<T: GetTableName> {
    i: T::Idx,
    end: T::Idx,
}
impl<T: GetTableName> UncheckedIter<T> {
    pub fn skip_fast(mut self, n: usize) -> Self {
        self.i = (T::Idx::from_usize(self.i.to_usize().unwrap() + n)).unwrap();
        self
    }
}
impl<T: GetTableName> Iterator for UncheckedIter<T> {
    type Item = GenericRowId<T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end {
            None
        } else {
            let ret = GenericRowId {
                i: self.i,
                table: PhantomData,
            };
            self.i = ret.next().i;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let s = self.end.to_usize().unwrap() - self.i.to_usize().unwrap();
        (s, Some(s))
    }
}




pub trait Checkable {
    type Row: GetTableName;
    fn check<'a, L>(self, table: &'a L) -> CheckedRowId<'a, L>
    where L: LockedTable<Row=Self::Row>;
    fn uncheck(self) -> GenericRowId<Self::Row>;
}
impl<T: GetTableName> Checkable for GenericRowId<T> {
    type Row = T;
    fn check<L>(self, table: &L) -> CheckedRowId<L>
    where L: LockedTable<Row=Self::Row>
    {
        let i = self.i;
        if i.to_usize().unwrap() >= table.len() {
            panic!("index out of bounds on table {}: the len is {}, but the index is {}",
                   T::get_name(), table.len(), i);
        }
        if table.is_deleted(self) {
            panic!("indexing on table {} into deleted row {}",
                   T::get_name(), i);
        }
        unsafe {
            CheckedRowId::fab(i, table)
        }
    }

    fn uncheck(self) -> GenericRowId<T> { self }
}
impl<'a, T: LockedTable + 'a> Checkable for CheckedRowId<'a, T> {
    type Row = T::Row;
    fn check<'c, L>(self, table: &'c L) -> CheckedRowId<'c, L>
    where L: LockedTable<Row=Self::Row>
    {
        if cfg!(debug) && self.table as *const T as usize != table as *const L as usize {
            panic!("mismatched tables");
        }
        CheckedRowId {
            i: self.i,
            table,
        }
    }
    fn uncheck(self) -> GenericRowId<T::Row> { GenericRowId::new(self.i) }
}


use crate::joincore::{JoinCore, Join};
use std::collections::btree_map;

pub type FreeList<T> = btree_map::BTreeMap<GenericRowId<T>, ()>;
pub type FreeKeys<'a, T> = btree_map::Keys<'a, GenericRowId<T>, ()>;

/// A `CheckedIter` that skips deleted rows.
pub struct ConsistentIter<'a, T: LockedTable + 'a> {
    rows: CheckedIter<'a, T>,
    deleted: JoinCore<FreeKeys<'a, T::Row>>,
}
impl<'a, T: LockedTable + 'a> ConsistentIter<'a, T> {
    pub fn new(rows: CheckedIter<'a, T>, deleted: &'a FreeList<T::Row>) -> Self {
        Self {
            rows,
            deleted: JoinCore::new(deleted.keys()),
        }
    }

    pub fn with_deleted(self) -> CheckedIter<'a, T> {
        self.rows
    }

    pub fn skip_fast(mut self, n: usize) -> Self {
        self.rows = self.rows.skip_fast(n);
        self
    }
}
impl<'a, T: LockedTable + 'a> Iterator for ConsistentIter<'a, T> {
    type Item = CheckedRowId<'a, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(row) = self.rows.next() {
            match self.deleted.join(row, |l, r| l.uncheck().cmp(*r)) {
                // This join is a bit backwards.
                Join::Next | Join::Stop => return Some(row),
                Join::Match(_) => continue,
            }
        }
        None
    }
}



/// An [`UncheckedIter`] used for making non-structural edits to the table's data.
pub struct EditIter<'w, T: GetTableName + 'w> {
    range: UncheckedIter<T>,
    deleted: JoinCore<FreeKeys<'w, T>>,
}
impl<'w, T: GetTableName + 'w> EditIter<'w, T> {
    pub fn new(range: RowRange<GenericRowId<T>>, free_keys: FreeKeys<'w, T>) -> Self {
        EditIter {
            range: range.iter_slow(),
            deleted: JoinCore::new(free_keys),
        }
    }

    pub fn skip_fast(mut self, n: usize) -> Self {
        self.range = self.range.skip_fast(n);
        self
    }
}
impl<'w, T: GetTableName + 'w> Iterator for EditIter<'w, T> {
    type Item = GenericRowId<T>;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(row) = self.range.next() {
            match self.deleted.join(row, |l, r| l.cmp(r)) {
                // This join is a bit backwards.
                Join::Next | Join::Stop => return Some(row),
                Join::Match(_) => continue,
            }
        }
        None
    }
}

pub struct Deletable<'a, T: GetTableName + 'a> {
    inner: GenericRowId<T>,
    mark: &'a Cell<bool>,
}
impl<'a, T: GetTableName + 'a> Deref for Deletable<'a, T> {
    type Target = GenericRowId<T>;
    fn deref(&self) -> &Self::Target { &self.inner }
}
impl<'a, T: GetTableName + 'a> Deletable<'a, T> {
    pub fn delete(self) {
        self.mark.set(true);
    }
}

pub struct BagIter<'w, T: LockedTable + 'w> {
    #[doc(hidden)] pub table: &'w mut T,
    #[doc(hidden)] pub delete: Cell<bool>,
    #[doc(hidden)] pub i: usize,
    #[doc(hidden)] pub last: Option<usize>,
}
impl<'a, T: LockedTable + 'a> Iterator for BagIter<'a, T> {
    type Item = (&'a mut T, Deletable<'a, T::Row>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.table.len() {
            return None;
        }
        if self.delete.get() {
            self.delete.set(false);
            self.table.delete_row(GenericRowId::from_usize(self.last.unwrap()));
        } else {
            self.last = Some(self.i);
            self.i += 1;
        }
        let this_i = self.last.unwrap();
        let this_i = GenericRowId::from_usize(this_i);
        unsafe {
            // FIXME: IM IN A RUSH OK???
            // (Tho this API might not be tractible...)
            use std::mem;
            Some((
                mem::transmute(&mut *self.table),
                Deletable {
                    inner: this_i,
                    mark: mem::transmute(&self.delete),
                },
            ))
        }
    }
}
impl<'a, T: LockedTable + 'a> Drop for BagIter<'a, T> {
    fn drop(&mut self) {
        if let (Some(i), true) = (self.last, self.delete.get()) {
            let i = GenericRowId::from_usize(i);
            self.table.delete_row(i);
        }
    }
}
