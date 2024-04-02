pub mod colors;
pub mod path_resolver;

pub fn varient_eq<T>(a: &T, b: &T) -> bool {
  std::mem::discriminant(a) == std::mem::discriminant(b)
}
