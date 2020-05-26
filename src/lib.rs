//! [![Crate](https://img.shields.io/crates/v/bumpy_vector.svg)](https://crates.io/crates/bumpy_vector)
//!
//! A vector-like object where elements can be larger than one item. We use
//! this primarily to represent objects in a binary that are made up of one
//! or more bytes.
//!
//! # Goal
//!
//! [h2gb](https://github.com/h2gb/libh2gb) is a tool for analyzing binary
//! files. Importantly, a binary file is a series of objects, each of which
//! take up some number of bytes. We need a datatype to represent this unusual
//! requirement, hence coming up with BumpyVector!
//!
//! # Usage
//!
//! Instantiate with a maximum size, then use somewhat like a vector:
//!
//! ```
//! use bumpy_vector::{BumpyEntry, BumpyVector};
//!
//! // Instantiate with a maximum size of 100 and a type of String
//! let mut v: BumpyVector<String> = BumpyVector::new(100);
//!
//! // Create a 10-byte entry at the start
//! let entry: BumpyEntry<String> = BumpyEntry {
//!   entry: String::from("hello"),
//!   size: 10,
//!   index: 0,
//! };
//!
//! // Insert it into the BumpyVector
//! assert!(v.insert(entry).is_ok());
//!
//! // Create another entry, this time from a tuple, that overlaps the first
//! let entry: BumpyEntry<String> = (String::from("error"), 1, 5).into();
//! assert!(v.insert(entry).is_err());
//!
//! // Create an entry that's off the end of the object
//! let entry: BumpyEntry<String> = (String::from("error"), 1000, 5).into();
//! assert!(v.insert(entry).is_err());
//!
//! // There is still one entry in this vector
//! assert_eq!(1, v.len());
//! ```
//!
//! # Serialize / deserialize
//!
//! When installed with the 'serialize' feature:
//!
//! ```toml
//! bumpy_vector = { version = "~0.0.0", features = ["serialize"] }
//! ```
//!
//! Serialization support using [serde](https://serde.rs/) is enabled. The
//! `BumpyVector` can be serialized with any of the serializers that Serde
//! supports, such as [ron](https://github.com/ron-rs/ron):
//!
//! ```ignore
//! use bumpy_vector::BumpyVector;
//!
//! // Assumes "serialize" feature is enabled: `bumpy_vector = { features = ["serialize"] }`
//! fn main() {
//!     let mut h: BumpyVector<String> = BumpyVector::new(10);
//!     h.insert((String::from("a"), 1, 2).into()).unwrap();
//!
//!     // Serialize
//!     let serialized = ron::ser::to_string(&h).unwrap();
//!
//!     // Deserialize
//!     let h: BumpyVector<String> = ron::de::from_str(&serialized).unwrap();
//! }
//! ```

use std::collections::HashMap;

#[cfg(feature = "serialize")]
use serde::{Serialize, Deserialize};

/// Represents a single entry.
///
/// An entry is comprised of an object of type `T`, a starting index, and a
/// size.
///
/// # Example 1
///
/// Creating a basic entry is very straight forward:
///
/// ```
/// use bumpy_vector::BumpyEntry;
///
/// let e: BumpyEntry<&str> = BumpyEntry {
///   entry: "hello",
///   index: 0,
///   size: 1,
/// };
/// ```
///
/// # Example 2
///
/// For convenience, you can create an entry from a (T, usize, usize) tuple:
///
/// ```
/// use bumpy_vector::BumpyEntry;
///
/// let e: BumpyEntry<&str> = ("hello", 0, 1).into();
/// ```
#[derive(Debug, Default)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct BumpyEntry<T> {
    pub entry: T,
    pub index: usize,
    pub size: usize,
}

impl<T> From<(T, usize, usize)> for BumpyEntry<T> {
    fn from(o: (T, usize, usize)) -> Self {
        BumpyEntry {
          entry: o.0,
          index: o.1,
          size: o.2,
        }
    }
}

/// Represents an instance of a Bumpy Vector
#[derive(Debug, Default)]
#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
pub struct BumpyVector<T> {
    /// The data is represented by a HashMap, where the index is the key and
    /// a BumpyEntry is the object.
    data: HashMap<usize, BumpyEntry<T>>,

    /// The maximum size.
    max_size: usize,

    /// If set, `into_iter()` will iterate over empty addresses.
    pub iterate_over_empty: bool,
}

/// Implement the object.
impl<'a, T> BumpyVector<T> {
    /// Create a new instance of BumpyVector.
    ///
    /// The range of the vector goes from `0` to `max_size - 1`. If any
    /// elements beyond the end are accessed, an error will be returned.
    pub fn new(max_size: usize) -> Self {
        BumpyVector {
            data: HashMap::new(),
            max_size: max_size,
            iterate_over_empty: false,
        }
    }

    /// Get the object that starts at or overlaps the starting index.
    ///
    /// This private method is the core of BumpyVector. Given an arbitrary
    /// offset within the BumpyVector, determine which entry exists in it (even
    /// if the entry starts to the "left").
    ///
    /// The initial implementation is somewhat naive: loop from the
    /// `starting_index` to 0, searching for an object. If found, check the
    /// object's size to ensure it overlaps the `starting_index`.
    ///
    /// This will be a good place to optimize later.
    fn get_entry_start(&self, starting_index: usize) -> Option<usize> {
        // Keep a handle to the starting index
        let mut index = starting_index;

        // Loop right to zero
        loop {
            // Check if we have data at the index
            match self.data.get(&index) {
                // If there's a value, we're set!
                Some(d) => {
                    // If we were too far away, it doesn't count. No value!
                    if d.size <= (starting_index - index) {
                        return None;
                    }

                    // Otherwise, we have the real index!
                    return Some(index);
                },

                // If there's no value, we keep going
                None => {
                    if index == 0 {
                        return None;
                    }

                    index -= 1;
                },
            };
        }
    }

    /// Insert a new entry.
    ///
    /// # Return
    ///
    /// Returns `Ok(())` if successfully inserted. If it would overlap another
    /// entry or exceed `max_size`, return `Err(&str)` with a descriptive error
    /// string.
    ///
    /// Size must be at least 1.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::{BumpyEntry, BumpyVector};
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert a 2-byte value starting at index 5 (using BumpyEntry directly)
    /// assert!(v.insert(BumpyEntry { entry: "hello", index: 5, size: 2 }).is_ok());
    ///
    /// // Insert another 2-byte value starting at index 7 (using into())
    /// assert!(v.insert(("hello", 7, 2).into()).is_ok());
    ///
    /// // Fail to insert a value that would overlap the first
    /// assert!(v.insert(("hello", 4, 2).into()).is_err());
    ///
    /// // Fail to insert a value that would overlap the second
    /// assert!(v.insert(("hello", 6, 1).into()).is_err());
    ///
    /// // Fail to insert a value that would go out of bounds
    /// assert!(v.insert(("hello", 100, 1).into()).is_err());
    /// ```
    pub fn insert(&mut self, entry: BumpyEntry<T>) -> Result<(), &'static str> {
        if entry.size == 0 {
            return Err("Zero is an invalid size for an entry");
        }

        if entry.index + entry.size > self.max_size {
            return Err("Invalid entry: entry exceeds max size");
        }

        // Check if there's a conflict on the left
        if self.get_entry_start(entry.index).is_some() {
            return Err("Invalid entry: overlaps another object");
        }

        // Check if there's a conflict on the right
        for x in entry.index..(entry.index + entry.size) {
            if self.data.contains_key(&x) {
                return Err("Invalid entry: overlaps another object");
            }
        }

        // We're good, so create an entry!
        self.data.insert(entry.index, entry);

        Ok(())
    }

    /// Remove and return the entry at `index`.
    ///
    /// Note that the entry doesn't necessarily need to *start* at `index`,
    /// just overlap it.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::BumpyVector;
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert some data
    /// v.insert(("hello", 0, 4).into()).unwrap();
    /// v.insert(("hello", 4, 4).into()).unwrap();
    ///
    /// assert!(v.remove(0).is_some());
    /// assert!(v.remove(0).is_none());
    ///
    /// assert!(v.remove(6).is_some());
    /// assert!(v.remove(6).is_none());
    /// ```
    pub fn remove(&mut self, index: usize) -> Option<BumpyEntry<T>> {
        // Try to get the real offset
        let real_offset = self.get_entry_start(index);

        // If there's no element, return none
        if let Some(o) = real_offset {
            // Remove it!
            if let Some(d) = self.data.remove(&o) {
                return Some(d);
            }
        }

        None
    }

    /// Remove and return a range of entries.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::BumpyVector;
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert some data
    /// v.insert(("hello", 0, 4).into()).unwrap();
    /// v.insert(("hello", 4, 4).into()).unwrap();
    ///
    /// assert_eq!(2, v.remove_range(0, 10).len());
    /// assert_eq!(0, v.remove_range(0, 10).len());
    /// ```
    pub fn remove_range(&mut self, index: usize, length: usize) -> Vec<BumpyEntry<T>> {
        let mut result: Vec<BumpyEntry<T>> = Vec::new();

        for i in index..(index+length) {
            if let Some(e) = self.remove(i) {
                result.push(e);
            }
        }

        result
    }

    /// Return a reference to an entry at the given index.
    ///
    /// Note that the entry doesn't necessarily need to *start* at the given
    /// index, it can simply be contained there.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::BumpyVector;
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert some data
    /// v.insert(("hello", 0, 4).into()).unwrap();
    ///
    /// assert!(v.get(0).is_some());
    /// assert!(v.get(1).is_some());
    /// assert!(v.get(2).is_some());
    /// assert!(v.get(3).is_some());
    /// assert!(v.get(4).is_none());
    /// assert!(v.get(5).is_none());
    ///
    /// assert_eq!(&"hello", v.get(0).unwrap().entry);
    /// assert_eq!(&"hello", v.get(1).unwrap().entry);
    /// assert_eq!(&"hello", v.get(2).unwrap().entry);
    /// assert_eq!(&"hello", v.get(3).unwrap().entry);
    /// ```
    pub fn get(&self, index: usize) -> Option<BumpyEntry<&T>> {
        // Try to get the real offset
        let real_offset = self.get_entry_start(index);

        // If there's no element, return none
        if let Some(o) = real_offset {
            // Get the entry itself from the address
            let entry = self.data.get(&o);

            // Although this probably won't fail, we need to check!
            if let Some(e) = entry {
                // Return the entry
                return Some(BumpyEntry {
                  entry: &e.entry,
                  index: e.index,
                  size: e.size,
                });
            }
        }

        None
    }

    /// Return a reference to an entry that *starts at* the given index.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::BumpyVector;
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert some data
    /// v.insert(("hello", 0, 4).into()).unwrap();
    ///
    /// assert!(v.get_exact(0).is_some());
    /// assert!(v.get_exact(1).is_none());
    /// assert!(v.get_exact(2).is_none());
    /// assert!(v.get_exact(3).is_none());
    /// assert!(v.get_exact(4).is_none());
    /// assert!(v.get_exact(5).is_none());
    ///
    /// assert_eq!(&"hello", v.get_exact(0).unwrap().entry);
    /// ```
    pub fn get_exact(&self, index: usize) -> Option<BumpyEntry<&T>> {
        match self.data.get(&index) {
            Some(e) => Some(BumpyEntry {
                entry: &e.entry,
                index: e.index,
                size: e.size,
            }),
            None    => None,
        }
    }

    /// Return a vector of entries within the given range.
    ///
    /// Note that the first entry doesn't need to *start* at the given index
    /// it can simply be contained there.
    ///
    /// # Parameters
    ///
    /// * `start` - The starting index.
    /// * `length` - The length to retrieve.
    /// * `include_empty` - If set, include empty entries in between the defined entries
    ///
    /// # Example
    ///
    /// ```
    /// use bumpy_vector::BumpyVector;
    ///
    /// // Create a 10-byte `BumpyVector`
    /// let mut v: BumpyVector<&str> = BumpyVector::new(10);
    ///
    /// // Insert some data with a gap in the middle
    /// v.insert(("hello", 0, 2).into()).unwrap();
    /// v.insert(("hello", 4, 2).into()).unwrap();
    ///
    /// // Don't include_empty:
    /// assert_eq!(1, v.get_range(0, 1, false).len());
    /// assert_eq!(1, v.get_range(0, 2, false).len());
    /// assert_eq!(1, v.get_range(0, 3, false).len());
    /// assert_eq!(1, v.get_range(0, 4, false).len());
    /// assert_eq!(2, v.get_range(0, 5, false).len());
    ///
    /// // Do include_empty:
    /// assert_eq!(1, v.get_range(0, 1, true).len());
    /// assert_eq!(1, v.get_range(0, 2, true).len());
    /// assert_eq!(2, v.get_range(0, 3, true).len());
    /// assert_eq!(3, v.get_range(0, 4, true).len());
    /// assert_eq!(4, v.get_range(0, 5, true).len());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if an entry's size is 0. That shouldn't be possible short of
    /// tinkering with internal state (most likely modifying serialized data).
    pub fn get_range(&self, start: usize, length: usize, include_empty: bool) -> Vec<BumpyEntry<Option<&T>>> {
        // We're stuffing all of our data into a vector to iterate over it
        let mut result: Vec<BumpyEntry<Option<&T>>> = Vec::new();

        // Start at the first entry left of what they wanted, if it exists
        let mut i = match self.get_entry_start(start) {
            Some(e) => e,
            None    => start,
        };

        // Loop up to <length> bytes after the starting index
        while i < start + length && i < self.max_size {
            // Pull the entry out, if it exists
            if let Some(e) = self.data.get(&i) {
                // Add the entry to the vector, and jump over it
                result.push(BumpyEntry {
                    entry: Some(&e.entry),
                    index: i,
                    size: e.size,
                });

                // Prevent an infinite loop
                if e.size == 0 {
                    panic!("Entry size cannot be 0!");
                }

                i += e.size;
            } else {
                // If the user wants empty elements, push i fake entry
                if include_empty {
                    result.push(BumpyEntry {
                      entry: None,
                      index: i,
                      size: 1,
                    });
                };
                i += 1;
            }
        }

        result
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        // Return the number of entries
        return self.data.len();
    }
}

/// Convert into an iterator.
///
/// Naively iterate across all entries, move them into a `Vec<_>`, and convert
/// that vector into an iterator.
///
impl<'a, T> IntoIterator for &'a BumpyVector<T> {
    type Item = BumpyEntry<Option<&'a T>>;
    type IntoIter = std::vec::IntoIter<BumpyEntry<Option<&'a T>>>;

    fn into_iter(self) -> std::vec::IntoIter<BumpyEntry<Option<&'a T>>> {
        return self.get_range(0, self.max_size, self.iterate_over_empty).into_iter();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_insert() {
        let mut h: BumpyVector<&str> = BumpyVector::new(100);

        // Insert a 5-byte value at 10
        h.insert(("hello", 10, 5).into()).unwrap();
        assert_eq!(1, h.len());

        // Earlier values are none
        assert!(h.get(8).is_none());
        assert!(h.get(9).is_none());

        // Middle values are all identical, no matter where in the entry we
        // retrieve it
        assert_eq!(&"hello", h.get(10).unwrap().entry);
        assert_eq!(10,       h.get(10).unwrap().index);
        assert_eq!(5,        h.get(10).unwrap().size);

        assert_eq!(&"hello", h.get(11).unwrap().entry);
        assert_eq!(10,       h.get(11).unwrap().index);
        assert_eq!(5,        h.get(11).unwrap().size);

        assert_eq!(&"hello", h.get(12).unwrap().entry);
        assert_eq!(10,       h.get(12).unwrap().index);
        assert_eq!(5,        h.get(12).unwrap().size);

        assert_eq!(&"hello", h.get(13).unwrap().entry);
        assert_eq!(10,       h.get(13).unwrap().index);
        assert_eq!(5,        h.get(13).unwrap().size);

        assert_eq!(&"hello", h.get(14).unwrap().entry);
        assert_eq!(10,       h.get(14).unwrap().index);
        assert_eq!(5,        h.get(14).unwrap().size);

        // Last couple entries are none
        assert!(h.get(15).is_none());
        assert!(h.get(16).is_none());

        // There should still be a single entry
        assert_eq!(1, h.len());
    }

    #[test]
    fn test_zero_sized_insert() {
        let mut h: BumpyVector<&str> = BumpyVector::new(100);

        // Insert a 0-byte array
        assert!(h.insert(("hello", 10, 0).into()).is_err());
        assert_eq!(0, h.len());
    }

    #[test]
    fn test_overlapping_one_byte_inserts() {
        let mut h: BumpyVector<&str> = BumpyVector::new(100);

        // Insert a 2-byte value at 10
        h.insert(("hello", 10, 2).into()).unwrap();
        assert_eq!(1, h.len());

        // We can insert before
        assert!(h.insert(("ok", 8,  1).into()).is_ok());
        assert_eq!(2, h.len());
        assert!(h.insert(("ok", 9,  1).into()).is_ok());
        assert_eq!(3, h.len());

        // We can't insert within
        assert!(h.insert(("error", 10, 1).into()).is_err());
        assert!(h.insert(("error", 11, 1).into()).is_err());
        assert_eq!(3, h.len());

        // We can insert after
        assert!(h.insert(("ok", 12, 1).into()).is_ok());
        assert_eq!(4, h.len());
        assert!(h.insert(("ok", 13, 1).into()).is_ok());
        assert_eq!(5, h.len());
    }

    #[test]
    fn test_overlapping_multi_byte_inserts() {
        // Define 10-12, put something at 7-9 (good!)
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 10, 3).into()).unwrap();
        assert!(h.insert(("ok", 7,  3).into()).is_ok());

        // Define 10-12, try every overlapping bit
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(BumpyEntry::from(("hello", 10, 3))).unwrap();
        assert!(h.insert(("error", 8,  3).into()).is_err());
        assert!(h.insert(("error", 9,  3).into()).is_err());
        assert!(h.insert(("error", 10, 3).into()).is_err());
        assert!(h.insert(("error", 11, 3).into()).is_err());
        assert!(h.insert(("error", 12, 3).into()).is_err());

        // 6-9 and 13-15 will work
        assert!(h.insert(BumpyEntry::from(("ok", 6,  3))).is_ok());
        assert!(h.insert(("ok", 13, 3).into()).is_ok());
        assert_eq!(3, h.len());
    }

    #[test]
    fn test_remove() {
        // Define 10-12, put something at 7-9 (good!)
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();
        h.insert(("hello", 10, 2).into()).unwrap();
        h.insert(("hello", 12, 2).into()).unwrap();
        assert_eq!(3, h.len());

        // Remove from the start of an entry
        let e = h.remove(10).unwrap();
        assert_eq!("hello", e.entry);
        assert_eq!(10,      e.index);
        assert_eq!(2,       e.size);
        assert_eq!(2,       h.len());
        assert!(h.get(10).is_none());
        assert!(h.get(11).is_none());

        // Put it back
        h.insert(("hello", 10, 2).into()).unwrap();
        assert_eq!(3, h.len());

        // Remove from the middle of an entry
        let e = h.remove(11).unwrap();
        assert_eq!("hello", e.entry);
        assert_eq!(10,      e.index);
        assert_eq!(2,       e.size);
        assert_eq!(2,       h.len());
        assert!(h.get(10).is_none());
        assert!(h.get(11).is_none());

        // Remove 11 again, which is nothing
        let result = h.remove(11);
        assert!(result.is_none());

        let e = h.remove(13).unwrap();
        assert_eq!("hello", e.entry);
        assert_eq!(12,      e.index);
        assert_eq!(2,       e.size);
        assert_eq!(1,       h.len());
        assert!(h.get(12).is_none());
        assert!(h.get(13).is_none());

        h.remove(8);
        assert_eq!(0, h.len());
        assert!(h.get(8).is_none());
        assert!(h.get(9).is_none());
    }

    #[test]
    fn test_beginning() {
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("hello", 0, 2).into()).unwrap();

        assert_eq!(1, h.len());

        assert_eq!(&"hello", h.get(0).unwrap().entry);
        assert_eq!(0,        h.get(0).unwrap().index);
        assert_eq!(2,        h.get(0).unwrap().size);

        assert_eq!(&"hello", h.get(1).unwrap().entry);
        assert_eq!(0,        h.get(1).unwrap().index);
        assert_eq!(2,        h.get(1).unwrap().size);

        assert!(h.get(2).is_none());
    }

    #[test]
    fn test_max_size() {
        // Inserting at 7-8-9 works
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("hello", 7, 3).into()).unwrap();
        assert_eq!(1, h.len());

        // Inserting at 8-9-10 and onward does not
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        assert!(h.insert(("hello", 8, 3).into()).is_err());
        assert_eq!(0, h.len());

        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        assert!(h.insert(("hello", 9, 3).into()).is_err());
        assert_eq!(0, h.len());

        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        assert!(h.insert(("hello", 10, 3).into()).is_err());
        assert_eq!(0, h.len());

        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        assert!(h.insert(("hello", 11, 3).into()).is_err());
        assert_eq!(0, h.len());
    }

    #[test]
    fn test_remove_range() {
        // Create an object
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();
        h.insert(("hello", 10, 2).into()).unwrap();
        h.insert(("hello", 12, 2).into()).unwrap();
        assert_eq!(3, h.len());

        // Test removing the first two entries
        let result = h.remove_range(8, 4);
        assert_eq!(1, h.len());
        assert_eq!(2, result.len());

        assert_eq!("hello", result[0].entry);
        assert_eq!(8,       result[0].index);
        assert_eq!(2,       result[0].size);

        assert_eq!("hello", result[1].entry);
        assert_eq!(10,      result[1].index);
        assert_eq!(2,       result[1].size);

        // Re-create the object
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();
        h.insert(("hello", 10, 2).into()).unwrap();
        h.insert(("hello", 12, 2).into()).unwrap();
        assert_eq!(3, h.len());

        // Test where the first entry starts left of the actual starting index
        let result = h.remove_range(9, 2);
        assert_eq!(1, h.len());
        assert_eq!(2, result.len());

        assert_eq!("hello", result[0].entry);
        assert_eq!(8,       result[0].index);
        assert_eq!(2,       result[0].size);

        assert_eq!("hello", result[1].entry);
        assert_eq!(10,      result[1].index);
        assert_eq!(2,       result[1].size);

        // Re-create the object
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();
        h.insert(("hello", 10, 2).into()).unwrap();
        h.insert(("hello", 12, 2).into()).unwrap();
        assert_eq!(3, h.len());

        // Test the entire object
        let result = h.remove_range(0, 1000);
        assert_eq!(0, h.len());
        assert_eq!(3, result.len());

        assert_eq!("hello", result[0].entry);
        assert_eq!(8,       result[0].index);
        assert_eq!(2,       result[0].size);

        assert_eq!("hello", result[1].entry);
        assert_eq!(10,      result[1].index);
        assert_eq!(2,       result[1].size);
    }

    #[test]
    fn test_get() {
        // Create an object
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();

        // Test removing the first two entries
        assert!(h.get(7).is_none());
        assert!(h.get(8).is_some());
        assert!(h.get(9).is_some());
        assert!(h.get(10).is_none());
    }

    #[test]
    fn test_get_exact() {
        // Create an object
        let mut h: BumpyVector<&str> = BumpyVector::new(100);
        h.insert(("hello", 8, 2).into()).unwrap();

        // Test removing the first two entries
        assert!(h.get_exact(7).is_none());
        assert!(h.get_exact(8).is_some());
        assert!(h.get_exact(9).is_none());
        assert!(h.get_exact(10).is_none());
    }

    #[test]
    fn test_get_range_skip_empty() {
        // Create a BumpyVector that looks like:
        //
        // [--0-- --1-- --2-- --3-- --4-- --5-- --6-- --7-- --8-- --9--]
        //        +-----------------            +----------------+
        //        |   "a" (2)| "b" |            |      "c"       |
        //        +----------+------            +----------------+
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("a", 1, 2).into()).unwrap();
        h.insert(("b", 3, 1).into()).unwrap();
        h.insert(("c", 6, 3).into()).unwrap();

        // Get just the first two
        let result = h.get_range(2, 4, false);
        assert_eq!(2, result.len());

        // Get the first two, then just barely the third
        let result = h.get_range(2, 5, false);
        assert_eq!(3, result.len());

        // Get the first two again, starting further left
        let result = h.get_range(1, 5, false);
        assert_eq!(2, result.len());

        // Get all three again
        let result = h.get_range(1, 6, false);
        assert_eq!(3, result.len());

        // Get way more than everything
        let result = h.get_range(0, 100, false);
        assert_eq!(3, result.len());
    }

    #[test]
    fn test_get_range_include_empty() {
        // Create a BumpyVector that looks like:
        //
        // [--0-- --1-- --2-- --3-- --4-- --5-- --6-- --7-- --8-- --9--]
        //        +-----------------            +----------------+
        //        |   "a" (2)| "b" |            |      "c"       |
        //        +----------+------            +----------------+
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("a", 1, 2).into()).unwrap();
        h.insert(("b", 3, 1).into()).unwrap();
        h.insert(("c", 6, 3).into()).unwrap();

        // Get just the first two, plus two empty spots
        let result = h.get_range(2, 4, true);
        assert_eq!(4, result.len());

        // Get the first two, the two empty spots, then just barely the third
        let result = h.get_range(2, 5, true);
        assert_eq!(5, result.len());

        // Get an empty spot, then the first one
        let result = h.get_range(0, 3, true);
        assert_eq!(2, result.len());

        // Get an empty spot, then the first two
        let result = h.get_range(0, 4, true);
        assert_eq!(3, result.len());

        // Get the last one, then the empty spot after it, then we're at the end and should stop
        let result = h.get_range(8, 1000, true);
        assert_eq!(2, result.len());
    }

    #[test]
    fn test_iterator_with_empty() {
        // Create a BumpyVector that looks like:
        //
        // [--0-- --1-- --2-- --3-- --4-- --5-- --6-- --7-- --8-- --9--]
        //        +-----------------            +----------------+
        //        |   "a" (2)| "b" |            |      "c"       |
        //        +----------+------            +----------------+
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("a", 1, 2).into()).unwrap();
        h.insert(("b", 3, 1).into()).unwrap();
        h.insert(("c", 6, 3).into()).unwrap();

        // Iterate over everything, including empty values
        h.iterate_over_empty = true;
        let mut iter = h.into_iter();

        // None (index 0)
        let e = iter.next().unwrap();
        assert!(e.entry.is_none());
        assert_eq!(0, e.index);
        assert_eq!(1, e.size);

        // Entry "a" (index 1-2)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"a"), e.entry);
        assert_eq!(1,          e.index);
        assert_eq!(2,          e.size);

        // Entry "b" (index 3)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"b"), e.entry);
        assert_eq!(3,          e.index);
        assert_eq!(1,          e.size);

        // None (index 4)
        let e = iter.next().unwrap();
        assert!(e.entry.is_none());
        assert_eq!(4, e.index);
        assert_eq!(1, e.size);

        // None (index 5)
        let e = iter.next().unwrap();
        assert!(e.entry.is_none());
        assert_eq!(5, e.index);
        assert_eq!(1, e.size);

        // Entry "c" (index 6-8)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"c"), e.entry);
        assert_eq!(6,          e.index);
        assert_eq!(3,          e.size);

        // None (index 9)
        let e = iter.next().unwrap();
        assert!(e.entry.is_none());
        assert_eq!(9, e.index);
        assert_eq!(1, e.size);

        // That's it!
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_iterator_skip_empty() {
        // Create a BumpyVector that looks like:
        //
        // [--0-- --1-- --2-- --3-- --4-- --5-- --6-- --7-- --8-- --9--]
        //        +-----------------            +----------------+
        //        |   "a" (2)| "b" |            |      "c"       |
        //        +----------+------            +----------------+
        let mut h: BumpyVector<&str> = BumpyVector::new(10);
        h.insert(("a", 1, 2).into()).unwrap();
        h.insert(("b", 3, 1).into()).unwrap();
        h.insert(("c", 6, 3).into()).unwrap();

        // Iterate over entries only, skip empty
        h.iterate_over_empty = false;
        let mut iter = h.into_iter();

        // Entry "a" (index 1-2)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"a"), e.entry);
        assert_eq!(1,          e.index);
        assert_eq!(2,          e.size);

        // Entry "b" (index 3)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"b"), e.entry);
        assert_eq!(3,          e.index);
        assert_eq!(1,          e.size);

        // Entry "c" (index 6-8)
        let e = iter.next().unwrap();
        assert_eq!(Some(&"c"), e.entry);
        assert_eq!(6,          e.index);
        assert_eq!(3,          e.size);

        // That's it!
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }

    #[test]
    #[cfg(feature = "serialize")] // Only test if we enable serialization
    fn test_serialize() {
        let mut h: BumpyVector<String> = BumpyVector::new(10);
        h.insert((String::from("a"), 1, 2).into()).unwrap();
        h.insert((String::from("b"), 3, 1).into()).unwrap();
        h.insert((String::from("c"), 6, 3).into()).unwrap();

        // Serialize
        let serialized = ron::ser::to_string(&h).unwrap();

        // Deserialize
        let h: BumpyVector<String> = ron::de::from_str(&serialized).unwrap();

        // Make sure we have the same entries
        assert_eq!("a", h.get(2).unwrap().entry);
        assert_eq!(1,   h.get(2).unwrap().index);
        assert_eq!(2,   h.get(2).unwrap().size);
        assert_eq!("b", h.get(3).unwrap().entry);
        assert!(h.get(4).is_none());
        assert!(h.get(5).is_none());
        assert_eq!("c", h.get(6).unwrap().entry);
        assert_eq!(6,   h.get(6).unwrap().index);
        assert_eq!(3,   h.get(6).unwrap().size);
    }
}
