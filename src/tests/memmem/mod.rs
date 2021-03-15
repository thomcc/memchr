#[cfg(all(feature = "std", not(miri)))]
mod properties;
#[cfg(feature = "std")]
mod simple;
