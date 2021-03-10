use crate::core::{FinitePeriod, Instant, Period, PeriodFrom};
use std::collections::BTreeMap;

pub trait TemporalRecord: Sized + private::SealedTemporalRecord {
  fn value(&self) -> &Self::Value;

  fn into_value(self) -> Self::Value;
}

mod private {
  use super::*;

  pub trait SealedTemporalRecord: Sized {
    type Value: Eq;

    fn from_value_and_time(value: Self::Value, time: Instant) -> Self;

    fn try_update(&mut self, other: Self) -> Result<(), Self>;

    fn latest_time(&self, start_time: Instant) -> Instant;
  }
}

impl<T: Eq> TemporalRecord for T {
  fn value(&self) -> &T {
    self
  }

  fn into_value(self) -> T {
    self
  }
}

impl<T: Eq> private::SealedTemporalRecord for T {
  type Value = T;

  fn from_value_and_time(value: T, _time: Instant) -> T {
    value
  }

  fn try_update(&mut self, other: T) -> Result<(), T> {
    if *self == other {
      Ok(())
    } else {
      Err(other)
    }
  }

  fn latest_time(&self, start_time: Instant) -> Instant {
    start_time
  }
}

#[derive(Copy, Clone, Debug, Hash)]
struct ArchivedRecord<T> {
  value: T,
  last_seen: Instant,
}

impl<T: Eq> TemporalRecord for ArchivedRecord<T> {
  fn value(&self) -> &T {
    &self.value
  }

  fn into_value(self) -> T {
    self.value
  }
}

impl<T: Eq> private::SealedTemporalRecord for ArchivedRecord<T> {
  type Value = T;

  fn from_value_and_time(value: T, time: Instant) -> Self {
    Self { value, last_seen: time }
  }

  fn try_update(&mut self, other: Self) -> Result<(), Self> {
    if self.value == other.value {
      if self.last_seen < other.last_seen {
        self.last_seen = other.last_seen;
      }
      Ok(())
    } else {
      Err(other)
    }
  }

  fn latest_time(&self, _start_time: Instant) -> Instant {
    self.last_seen
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnapshotFrom<T> {
  period: PeriodFrom,
  record: T,
}

impl<T> SnapshotFrom<T> {
  pub fn new(start: Instant, record: T) -> Self {
    Self {
      period: (start..).into(),
      record,
    }
  }

  pub fn period(&self) -> PeriodFrom {
    self.period
  }

  pub fn start_time(&self) -> Instant {
    self.period.start
  }

  pub fn value(&self) -> &T::Value
  where
    T: TemporalRecord,
  {
    self.record.value()
  }

  pub fn into_value(self) -> T::Value
  where
    T: TemporalRecord,
  {
    self.record.into_value()
  }

  pub fn record(&self) -> &T {
    &self.record
  }

  pub fn into_record(self) -> T {
    self.record
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Snapshot<T> {
  period: Period,
  record: T,
}

impl<T> Snapshot<T> {
  pub fn period(&self) -> Period {
    self.period
  }

  pub fn start_time(&self) -> Instant {
    match self.period {
      Period::From(PeriodFrom { start }) => start,
      Period::Finite(FinitePeriod { start, .. }) => start,
    }
  }

  pub fn end_time(&self) -> Option<Instant> {
    match self.period {
      Period::From(_) => None,
      Period::Finite(FinitePeriod { end, .. }) => Some(end),
    }
  }

  pub fn value(&self) -> &T::Value
  where
    T: TemporalRecord,
  {
    self.record.value()
  }

  pub fn into_value(self) -> T::Value
  where
    T: TemporalRecord,
  {
    self.record.into_value()
  }

  pub fn record(&self) -> &T {
    &self.record
  }

  pub fn into_record(self) -> T {
    self.record
  }
}

pub struct Temporal<T> {
  current: SnapshotFrom<T>,
  old: BTreeMap<Instant, T>,
}

impl<T> Temporal<T> {
  pub fn new(time: Instant, record: T) -> Self {
    Self {
      current: SnapshotFrom::new(time, record),
      old: BTreeMap::new(),
    }
  }

  pub fn at(&self, time: Option<Instant>) -> Option<SnapshotFrom<&T>> {
    let time = time.unwrap_or(self.current.period.start);
    if time >= self.current.start_time() {
      Some(SnapshotFrom::new(self.current.start_time(), &self.current.record))
    } else {
      self
        .old
        .range(..=time)
        .rev()
        .next()
        .map(|(t, v)| SnapshotFrom::new(*t, v))
    }
  }

  pub fn time(&self) -> Instant {
    self.current.start_time()
  }

  pub fn map<B: TemporalRecord, F: FnMut(Snapshot<&T>) -> B>(&self, mut f: F) -> Temporal<B> {
    let mut it = self
      .old
      .iter()
      .chain(core::iter::once((&self.current.period.start, &self.current.record)))
      .peekable();

    let mut mapped: Option<Temporal<B>> = None;

    while let Some((start, r)) = it.next() {
      let end = it.peek().map(|(t, ..)| *t);
      let period = match end {
        Some(end) => Period::Finite(FinitePeriod {
          start: *start,
          end: *end,
        }),
        None => Period::From(PeriodFrom { start: *start }),
      };
      let r = f(Snapshot { period, record: r });
      mapped = match mapped {
        None => Some(Temporal::new(*start, r)),
        Some(mut m) => {
          m.set_record(*start, r);
          Some(m)
        }
      }
    }

    mapped.unwrap()
  }

  pub fn iter(&self) -> impl Iterator<Item = Snapshot<&T>> {
    let mut next: Option<Instant> = None;
    self
      .old
      .iter()
      .chain(core::iter::once((&self.current.period.start, &self.current.record)))
      .rev()
      .map(move |(start, r)| {
        let period = match next {
          Some(end) => Period::Finite(FinitePeriod { start: *start, end }),
          None => Period::From(PeriodFrom { start: *start }),
        };
        next = Some(*start);
        Snapshot { period, record: r }
      })
      .rev()
  }
}

impl<T: TemporalRecord> Temporal<T> {
  pub fn value(&self) -> &T::Value {
    &self.current.value()
  }

  pub fn into_value(self) -> T::Value {
    self.current.into_value()
  }

  pub fn set(&mut self, time: Instant, value: T::Value) {
    let record = T::from_value_and_time(value, time);
    self.set_record(time, record);
  }

  fn set_record(&mut self, time: Instant, record: T) {
    let new_time = record.latest_time(time);
    let old_time = self.current.record.latest_time(self.current.period.start);
    assert!(new_time > old_time, "Values must be provided in chronological order");

    if let Err(record) = self.current.record.try_update(record) {
      let next = SnapshotFrom {
        period: (time..).into(),
        record,
      };
      let prev = core::mem::replace(&mut self.current, next);
      let old = self.old.insert(prev.period.start, prev.record);
      debug_assert!(old.is_none());
    }
  }

  pub fn from_values<I: IntoIterator<Item = (Instant, T)>>(iter: I) -> Self {
    let mut iter = iter.into_iter();
    let mut cur = iter.next().expect("Cannot construct Temporal<T> from empty iterator");
    assert!(cur.0 <= cur.1.latest_time(cur.0), "Invalid start time for value");

    let mut old = BTreeMap::<Instant, T>::new();
    for (next_time, next_val) in iter {
      let latest_time = next_val.latest_time(next_time);
      assert!(next_time <= latest_time, "Invalid start time for value");
      assert!(latest_time > next_time, "Values must be provided in cronological order");

      if let Err(next_val) = cur.1.try_update(next_val) {
        let prev = core::mem::replace(&mut cur, (next_time, next_val));
        let old = old.insert(prev.0, prev.1);
        debug_assert!(old.is_none());
      }
    }

    Temporal {
      current: SnapshotFrom::new(cur.0, cur.1),
      old,
    }
  }
}
