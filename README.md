# bumpy_vector

A vector-like object where elements can be larger than one "space". We use
this primarily to represent objects in a binary that are made up of one
or more bytes.

## Goal

[h2gb](https://github.com/h2gb/libh2gb) is a tool for analyzing binary
files. Importantly, a binary file is a series of objects, each of which
take up some number of bytes. We need a datatype to represent this unusual
requirement, hence coming up with BumpyVector!

## Usage

Instantiate with a maximum size, then use somewhat like a vector:

```rust
use bumpy_vector::{BumpyEntry, BumpyVector};

// Instantiate with a maximum size of 100 and a type of String
let mut v: BumpyVector<String> = BumpyVector::new(100);

// Create a 10-byte entry at the start
let entry: BumpyEntry<String> = BumpyEntry {
  entry: String::from("hello"),
  size: 10,
  index: 0,
};

// Insert it into the BumpyVector
assert!(v.insert(entry).is_ok());

// Create another entry, this time from a tuple, that overlaps the first
let entry: BumpyEntry<String> = (String::from("error"), 1, 5).into();
assert!(v.insert(entry).is_err());

// Create an entry that's off the end of the object
let entry: BumpyEntry<String> = (String::from("error"), 1000, 5).into();
assert!(v.insert(entry).is_err());

// There is still one entry in this vector
assert_eq!(1, v.len());
```

## TODO
* Handle 0-sized objects better (well, error sooner)

License: MIT
