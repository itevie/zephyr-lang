# For
- Used for iterating over an interable value

## Syntax
```
for x, y in v { }
```
- Where `x` is a Symbol meaning the current index of the iteration
- Where `y` is a Symbol meaning the current value of the interation
- Where `v` is a value which is iterable
- `, y` is optional if you don't need the value

## Examples
```
for i, v in [1, 2, 3] {
  print(i, v);
};
// Output:
// 0, 1
// 1, 2
// 2, 3
```
