# Assign
- Used to assign to a variable

## Syntax
`x = y`
- Where `x` is a `Symbol` or `Member`
  - `Symbol` = Reference to variable
  - `Member` = Object member expression
- And `y` is the new value to assign to it

## Examples
```
let x = 2;
x = 3;
```
```
let x = .{
  b: 2
};
x.b = 3;
```
```
let x = [0, 1, 2];
x[0] = 2;
```
