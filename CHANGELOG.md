# Version 0.0.0

Initial codebase.

# Version 0.0.1

Changes:
* Much better documentation, including a README.md file [#1]
* Added git pre-commit hooks [#1]
* Serialization is disabled by default behind a feature [#2]
* Don't infinite loop when a zero-length entry ends up in the vector [#3]

# Version 0.0.2

Changes:
* Added `#[derive(Clone)]` to make it cloneable [#7]

# Version 0.0.3

Changes:
* Added `.max_size()` function [#10]
* `.get()` and `.get_exact()` now return a reference to the `BumpyEntry` rather than a new `BumpyEntry` with a reference to the object `T` [#10]
* Add `.get_mut()` and `.get_exact_mut()` functions [#10]

# Version 0.0.4

Changes:
* Remove `iterate_over_empty` as an option [#11]
* Simplify iterator code [#11]

# Version 0.0.5

Changes:
* Add `trait AutoBumpyEntry`, to simplify entries that know their own
  size/index [#12]
* Change error handling to use `SimpleResult` [#13]
* Change the index..size stuff to use `std::ops::Range` [#14]
  * Note that this breaks compatibility
