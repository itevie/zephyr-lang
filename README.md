# Zephyr Lang

A dynamically typed interpreted language made in rust.

## Quick Info

This is my first proper project made in Rust so it will definetly not have the best practices.  
If you find any inefficient code that could be improved, or if you have and ideas, please open an issue.

To see the old C# version, go [here](https://github.com/itevie/ZephyrLanguage)

## Running

To run this on any platform (only Linux tested), simply run cargo run, this will land you in the REPL mode.  
To run a file run `cargo run -- run <file name>.zr`

## Quick Example

```
func reverse_array(array: array?) {
    let new_array = [];

    for i in $array-1..0 {
        new_array.push(array[i]);
    }

    return new_array;
}

reverse_array(1..5); // [5, 4, 3, 2, 1]
```

## Learning

At the moment, there won't be any tutorials or anything, to see working examples, look in "src/lib" or "examples", these contain the global variables for every Zephyr application.
