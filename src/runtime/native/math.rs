use crate::runtime::{
    native::add_native,
    values::{self, RuntimeValue, RuntimeValueUtils},
    R,
};

use std::sync::Arc;

use super::{make_no_args_error, NativeExecutionContext};

pub fn all() -> Vec<(String, RuntimeValue)> {
    vec![
        add_native!("math_sin", math_sin),
        add_native!("math_cos", math_cos),
        add_native!("math_tan", math_tan),
        add_native!("math_asin", math_asin),
        add_native!("math_acos", math_acos),
        add_native!("math_atan", math_atan),
        add_native!("math_sinh", math_sinh),
        add_native!("math_cosh", math_cosh),
        add_native!("math_tanh", math_tanh),
        add_native!("math_asinh", math_asinh),
        add_native!("math_acosh", math_acosh),
        add_native!("math_atanh", math_atanh),
        add_native!("math_exp", math_exp),
        add_native!("math_expm1", math_expm1),
        add_native!("math_log", math_log),
        add_native!("math_log1p", math_log1p),
        add_native!("math_log2", math_log2),
        add_native!("math_log10", math_log10),
        add_native!("math_sqrt", math_sqrt),
        add_native!("math_cbrt", math_cbrt),
        add_native!("math_abs", math_abs),
        add_native!("math_floor", math_floor),
        add_native!("math_ceil", math_ceil),
        add_native!("math_round", math_round),
        add_native!("math_trunc", math_trunc),
    ]
}

macro_rules! def_math_fn {
    ($name:ident, $fn:expr) => {
        pub fn $name(ctx: NativeExecutionContext) -> R {
            match &ctx.args[..] {
                [RuntimeValue::Number(num)] => Ok(values::Number::new($fn(num.value)).wrap()),
                _ => Err(make_no_args_error(ctx.location)),
            }
        }
    };
}

def_math_fn!(math_sin, f64::sin);
def_math_fn!(math_cos, f64::cos);
def_math_fn!(math_tan, f64::tan);
def_math_fn!(math_asin, f64::asin);
def_math_fn!(math_acos, f64::acos);
def_math_fn!(math_atan, f64::atan);
def_math_fn!(math_sinh, f64::sinh);
def_math_fn!(math_cosh, f64::cosh);
def_math_fn!(math_tanh, f64::tanh);
def_math_fn!(math_asinh, f64::asinh);
def_math_fn!(math_acosh, f64::acosh);
def_math_fn!(math_atanh, f64::atanh);
def_math_fn!(math_exp, f64::exp);
def_math_fn!(math_expm1, f64::exp_m1);
def_math_fn!(math_log, f64::ln);
def_math_fn!(math_log1p, f64::ln_1p);
def_math_fn!(math_log2, f64::log2);
def_math_fn!(math_log10, f64::log10);
def_math_fn!(math_sqrt, f64::sqrt);
def_math_fn!(math_cbrt, f64::cbrt);
def_math_fn!(math_abs, f64::abs);
def_math_fn!(math_floor, f64::floor);
def_math_fn!(math_ceil, f64::ceil);
def_math_fn!(math_round, f64::round);
def_math_fn!(math_trunc, f64::trunc);
