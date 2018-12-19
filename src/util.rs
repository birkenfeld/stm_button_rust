//! Utility macros/functions.

macro_rules! bitset {
    ($e:expr; $p:ident = true, $($tt:tt)+)    => { bitset!($e.$p().set_bit(); $($tt)+) };
    ($e:expr; $p:ident = false, $($tt:tt)+)   => { bitset!($e.$p().clear_bit(); $($tt)+) };
    ($e:expr; $p:ident = bit($v:expr), $($tt:tt)+) => { bitset!($e.$p().bit($v); $($tt)+) };
    ($e:expr; $p:ident = $v:expr, $($tt:tt)+) => { bitset!($e.$p().bits($v); $($tt)+) };
    ($e:expr; $p:ident = true)    => { $e.$p().set_bit() };
    ($e:expr; $p:ident = false)   => { $e.$p().clear_bit() };
    ($e:expr; $p:ident = bit($v:expr)) => { $e.$p().bit($v) };
    ($e:expr; $p:ident = $v:expr) => { $e.$p().bits($v) };
}

#[macro_export]
macro_rules! write {
    ($p:ident . $r:ident : $($tt:tt)+) => {
        unsafe { (*stm::$p::ptr()).$r.write(|w| bitset!(w; $($tt)+)); }
    };
}

#[macro_export]
macro_rules! read {
    ($p:ident . $r:ident : $bit:ident) => {
        unsafe { (*stm::$p::ptr()).$r.read().$bit().bits() }
    };
}

#[macro_export]
macro_rules! readb {
    ($p:ident . $r:ident : $bit:ident) => {
        unsafe { (*stm::$p::ptr()).$r.read().$bit().bit_is_set() }
    };
}

#[macro_export]
macro_rules! modif {
    ($p:ident . $r:ident : $($tt:tt)+) => {
        unsafe { (*stm::$p::ptr()).$r.modify(|_, w| bitset!(w; $($tt)+)); }
    };
}

#[macro_export]
macro_rules! wait_for {
    ($p:ident . $r:ident : $bit:ident) => {
        unsafe { while (*stm::$p::ptr()).$r.read().$bit().bit_is_clear() {} }
    };
    ($p:ident . $r:ident : ! $bit:ident) => {
        unsafe { while (*stm::$p::ptr()).$r.read().$bit().bit_is_set() {} }
    };
}
