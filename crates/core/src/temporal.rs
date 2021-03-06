use crate::core::{FinitePeriod, Instant, Period, PeriodFrom};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SnapshotFrom<T> {
  period: PeriodFrom,
  value: T,
}

impl<T> SnapshotFrom<T> {
  pub fn new(start: Instant, value: T) -> Self {
    Self {
      period: (start..).into(),
      value,
    }
  }

  pub fn period(&self) -> PeriodFrom {
    self.period
  }

  pub fn start_time(&self) -> Instant {
    self.period.start
  }

  pub fn value(&self) -> &T {
    &self.value
  }

  pub fn into_value(self) -> T {
    self.value
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Snapshot<T> {
  period: Period,
  value: T,
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

  pub fn value(&self) -> &T {
    &self.value
  }

  pub fn into_value(self) -> T {
    self.value
  }
}

pub struct Temporal<T> {
  current: SnapshotFrom<T>,
  old: BTreeMap<Instant, T>,
}

impl<T: Eq> Temporal<T> {
  pub fn new(time: Instant, value: T) -> Self {
    Self {
      current: SnapshotFrom::new(time, value),
      old: BTreeMap::new(),
    }
  }

  pub fn at(&self, time: Option<Instant>) -> Option<SnapshotFrom<&T>> {
    let time = time.unwrap_or(self.current.period.start);
    if time >= self.current.start_time() {
      Some(SnapshotFrom::new(self.current.start_time(), &self.current.value))
    } else {
      self
        .old
        .range(..=time)
        .rev()
        .next()
        .map(|(t, v)| SnapshotFrom::new(*t, v))
    }
  }

  pub fn set(&mut self, time: Instant, value: T) {
    assert!(time > self.current.period.start);
    if value != self.current.value {
      let next = SnapshotFrom {
        period: (time..).into(),
        value,
      };
      let prev = core::mem::replace(&mut self.current, next);
      let old = self.old.insert(prev.period.start, prev.value);
      debug_assert!(old.is_none());
    }
  }

  pub fn time(&self) -> Instant {
    self.current.start_time()
  }

  pub fn value(&self) -> &T {
    &self.current.value()
  }

  pub fn into_value(self) -> T {
    self.current.into_value()
  }

  pub fn map<B: Eq, F: FnMut(Snapshot<&T>) -> B>(&self, mut f: F) -> Temporal<B> {
    let mut it = self
      .old
      .iter()
      .chain(core::iter::once((&self.current.period.start, &self.current.value)))
      .peekable();

    let mut result: Option<Temporal<B>> = None;

    while let Some((start, v)) = it.next() {
      let end = it.peek().map(|(t, ..)| *t);
      let period = match end {
        Some(end) => Period::Finite(FinitePeriod {
          start: *start,
          end: *end,
        }),
        None => Period::From(PeriodFrom { start: *start }),
      };
      let v = f(Snapshot { period, value: v });
      result = match result {
        None => Some(Temporal::new(*start, v)),
        Some(mut r) => {
          r.set(*start, v);
          Some(r)
        }
      }
    }

    result.unwrap()
  }

  pub fn iter(&self) -> impl Iterator<Item = Snapshot<&T>> {
    let mut next: Option<Instant> = None;
    self
      .old
      .iter()
      .chain(core::iter::once((&self.current.period.start, &self.current.value)))
      .rev()
      .map(move |(start, v)| {
        let period = match next {
          Some(end) => Period::Finite(FinitePeriod { start: *start, end }),
          None => Period::From(PeriodFrom { start: *start }),
        };
        next = Some(*start);
        Snapshot { period, value: v }
      })
      .rev()
  }

  pub fn from_iter<I: IntoIterator<Item = (Instant, T)>>(iter: I) -> Self {
    let mut iter = iter.into_iter();
    let mut cur: (Instant, T) = match iter.next() {
      Some((t, v)) => (t, v),
      None => panic!("Cannot construct Temporal<T> from empty iterator"),
    };
    let mut old: BTreeMap<Instant, T> = BTreeMap::new();
    for new_cur in iter {
      assert!(new_cur.0 > cur.0);
      if new_cur.1 != cur.1 {
        let old_cur = core::mem::replace(&mut cur, new_cur);
        old.insert(old_cur.0, old_cur.1);
      }
    }
    Self {
      current: SnapshotFrom::new(cur.0, cur.1),
      old,
    }
  }
}

