# Function
- Functions are first-class literals, meaning they can be created like normal expressions

## Syntax
`func x(...) { ...2 }`
- Where `x` is a Symbol
- Where `...` is a comma-seperated Symbol list containing the names of parameters
- Where `...2` is the function's block

`func x { ... }`
- Same as before, but the parameter list is optional (as well as the parenthesis)

`let x = func x { ... }`
- The above ones are syntax sugar to this.

## Examples
```
func a {
  2;
}

a(); // 2
```
```
func a(b) {
  b;
}
a(4); // 4
```
```
func a(b, c) {
  b + c;
}
a(2, 3); // 5
```
```
func a(b) { b }(2); // 2
```
