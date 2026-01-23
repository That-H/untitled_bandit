use std::ops::{self, Deref};
use std::fmt;

/// Stores a value and ensures it does not exceed a maximum.
#[derive(Clone)]
pub struct Datum<T: Clone + PartialOrd> {
    /// Maximum value of the datum.
    pub max: T,
    cur: Option<T>,
}

impl<T: Clone + PartialOrd> Datum<T> {
    /// Create a new Datum with the given max value. The current value will also
    /// be set to this value.
    pub fn new(max: T) -> Self {
        Self { cur: None, max }
    }

    /// Set the current value to the given one if it is less than max.
    pub fn set_to(&mut self, new_val: T) {
        self.cur = if new_val > self.max {
            None
        } else {
            Some(new_val)
        }
    }

    /// Reset to the max value.
    pub fn reset(&mut self) {
        self.cur = None;
    }

    /// Change the maximum to a new value. Automatically lowers the current value if it is above
    /// the new maximum.
    pub fn change_max(&mut self, new_max: T) {
        if self.value() >= &new_max {
            self.reset();
        }
        self.max = new_max;
    }

    /// Return a reference to the current value stored. Equivalent to deref, but more explicit.
    pub fn value(&self) -> &T {
        self.deref()
    }
}

impl<T: Clone + PartialOrd> PartialEq<T> for Datum<T> {
    fn eq(&self, other: &T) -> bool {
        (**self) == *other
    }
}

impl<T: Clone + PartialOrd + fmt::Display> fmt::Display for Datum<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value().fmt(f)
    }
}

impl<T: Clone + PartialOrd> ops::AddAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Add<T, Output = T>,
{
    fn add_assign(&mut self, other: T) {
        self.set_to((*self).deref() + other)
    }
}

impl<T: Clone + PartialOrd> ops::SubAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Sub<T, Output = T>,
{
    fn sub_assign(&mut self, other: T) {
        self.set_to((*self).deref() - other)
    }
}

impl<T: Clone + PartialOrd> ops::MulAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Mul<T, Output = T>,
{
    fn mul_assign(&mut self, other: T) {
        self.set_to((*self).deref() * other)
    }
}

impl<T: Clone + PartialOrd> ops::DivAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Div<T, Output = T>,
{
    fn div_assign(&mut self, other: T) {
        self.set_to((*self).deref() / other)
    }
}

impl<T: Clone + PartialOrd> ops::Deref for Datum<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.cur.as_ref() {
            Some(c) => c,
            None => &self.max,
        }
    }
}
