//! Utility macros for working on registers.

macro_rules! bitset {
    ($e:expr; $p:ident = true, $($tt:tt)+)    => { bitset!($e.$p().set_bit(); $($tt)+) };
    ($e:expr; $p:ident = false, $($tt:tt)+)   => { bitset!($e.$p().clear_bit(); $($tt)+) };
    ($e:expr; $p:ident = bit($v:expr), $($tt:tt)+) => { bitset!($e.$p().bit($v); $($tt)+) };
    ($e:expr; $p:ident = @$v:ident, $($tt:tt)+) => { bitset!($e.$p().$v(); $($tt)+) };
    ($e:expr; $p:ident = $v:expr, $($tt:tt)+) => { bitset!($e.$p().bits($v); $($tt)+) };
    ($e:expr; $p:ident = true)    => { $e.$p().set_bit() };
    ($e:expr; $p:ident = false)   => { $e.$p().clear_bit() };
    ($e:expr; $p:ident = bit($v:expr)) => { $e.$p().bit($v) };
    ($e:expr; $p:ident = @$v:ident) => { $e.$p().$v() };
    ($e:expr; $p:ident = $v:expr) => { $e.$p().bits($v) };
}

#[macro_export]
macro_rules! write {
    ($p:ident . $($r:ident $([$ix:expr])?).+ : $($tt:tt)+) => {
        unsafe { (*stm::$p::ptr()).$($r$([$ix])?).+.write(|w| bitset!(w; $($tt)+)); }
    };
}

#[macro_export]
macro_rules! read {
    ($p:ident .  $($r:ident $([$ix:expr])?).+ : $bit:ident) => {
        unsafe { (*stm::$p::ptr()).$($r$([$ix])?).+.read().$bit().bits() }
    };
}

#[macro_export]
macro_rules! readr {
    ($p:ident .  $($r:ident $([$ix:expr])?).+) => {
        unsafe { (*stm::$p::ptr()).$($r$([$ix])?).+.read() }
    };
}

#[macro_export]
macro_rules! readb {
    ($p:ident .  $($r:ident $([$ix:expr])?).+ : $bit:ident) => {
        unsafe { (*stm::$p::ptr()).$($r$([$ix])?).+.read().$bit().bit_is_set() }
    };
}

#[macro_export]
macro_rules! modif {
    ($p:ident . $($r:ident $([$ix:expr])?).+ : $($tt:tt)+) => {
        unsafe { (*stm::$p::ptr()).$($r$([$ix])?).+.modify(|_, w| bitset!(w; $($tt)+)); }
    };
}

#[macro_export]
macro_rules! pulse {
    ($p:ident . $($r:ident $([$ix:expr])?).+ : $bit:ident) => {
        write!($p.$($r$([$ix])?).+: $bit = true);
        write!($p.$($r$([$ix])?).+: $bit = false);
    };
}

#[macro_export]
macro_rules! wait_for {
    ($p:ident . $($r:ident $([$ix:expr])?).+ : $bit:ident) => {
        unsafe { while (*stm::$p::ptr()).$($r$([$ix])?).+.read().$bit().bit_is_clear() {} }
    };
    ($p:ident . $($r:ident $([$ix:expr])?).+ : ! $bit:ident) => {
        unsafe { while (*stm::$p::ptr()).$($r$([$ix])?).+.read().$bit().bit_is_set() {} }
    };
}
