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

/// A versioned first-party value, whose value is always known.
pub struct Temporal<T> {
  current: SnapshotFrom<T>,
  old: BTreeMap<Instant, T>,
}

impl<T> Temporal<T> {
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
}

impl<T: Eq> Temporal<T> {
  pub fn set(&mut self, time: Instant, value: T) {
    assert!(
      time > self.current.period.start,
      "Values must be provided in chronological order"
    );
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

  pub fn from_values<I: IntoIterator<Item = (Instant, T)>>(iter: I) -> Self {
    let mut iter = iter.into_iter();
    let mut cur: (Instant, T) = match iter.next() {
      Some((t, v)) => (t, v),
      None => panic!("Cannot construct Temporal<T> from empty iterator"),
    };
    let mut old: BTreeMap<Instant, T> = BTreeMap::new();
    for new_cur in iter {
      assert!(new_cur.0 > cur.0, "Values must be provided in cronological order");
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchivedValue<T> {
  value: T,
  last_seen: Instant,
}

impl<T> ArchivedValue<T> {
  pub fn new(value: T, last_seen: Instant) -> Self {
    ArchivedValue { value, last_seen }
  }

  pub fn value(&self) -> &T {
    &self.value
  }

  pub fn into_value(self) -> T {
    self.value
  }

  pub fn last_seen(&self) -> Instant {
    self.last_seen
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchivedSnapshotFrom<T>(SnapshotFrom<ArchivedValue<T>>);

impl<T> ArchivedSnapshotFrom<T> {
  pub fn new(start: Instant, value: T, last_seen: Instant) -> Self {
    Self(SnapshotFrom::new(start, ArchivedValue::new(value, last_seen)))
  }

  pub fn period(&self) -> PeriodFrom {
    self.0.period()
  }

  pub fn start_time(&self) -> Instant {
    self.0.start_time()
  }

  pub fn last_seen(&self) -> Instant {
    self.0.value().last_seen
  }

  pub fn value(&self) -> &T {
    &self.0.value().value
  }

  pub fn into_value(self) -> T {
    self.0.into_value().value
  }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArchivedSnapshot<T>(Snapshot<ArchivedValue<T>>);

impl<T> ArchivedSnapshot<T> {
  pub fn period(&self) -> Period {
    self.0.period()
  }

  pub fn start_time(&self) -> Instant {
    self.0.start_time()
  }

  pub fn end_time(&self) -> Option<Instant> {
    self.0.end_time()
  }

  pub fn last_seen(&self) -> Instant {
    self.0.value().last_seen
  }

  pub fn value(&self) -> &T {
    &self.0.value().value
  }

  pub fn into_value(self) -> T {
    self.0.into_value().value
  }
}

/// A versioned third-party value, whose value may be unknown at certain times.
pub struct Archived<T>(Temporal<ArchivedValue<T>>);

impl<T> Archived<T> {
  pub fn new(time: Instant, value: T, last_seen: Instant) -> Self {
    Self(Temporal::new(time, ArchivedValue::new(value, last_seen)))
  }

  pub fn at(&self, time: Option<Instant>) -> Option<ArchivedSnapshotFrom<&T>> {
    self
      .0
      .at(time)
      .map(|s| ArchivedSnapshotFrom::new(s.start_time(), s.value().value(), s.value().last_seen()))
  }

  pub fn time(&self) -> Instant {
    self.0.time()
  }

  pub fn last_seen(&self) -> Instant {
    self.0.value().last_seen()
  }

  pub fn value(&self) -> &T {
    self.0.value().value()
  }

  pub fn into_value(self) -> T {
    self.0.into_value().into_value()
  }

  pub fn iter(&self) -> impl Iterator<Item = ArchivedSnapshot<&T>> {
    self.0.iter().map(|s| {
      ArchivedSnapshot(Snapshot {
        period: s.period,
        value: ArchivedValue {
          value: s.value().value(),
          last_seen: s.value().last_seen(),
        },
      })
    })
  }
}

impl<T: Eq> Archived<T> {
  pub fn set(&mut self, time: Instant, value: T) {
    assert!(
      time > self.last_seen(),
      "Values must be provided in chronological order"
    );
    if &value == self.value() {
      self.0.current.value.last_seen = time;
    } else {
      let next = SnapshotFrom::new(time, ArchivedValue::new(value, time));
      let prev = core::mem::replace(&mut self.0.current, next);
      let old = self.0.old.insert(prev.period.start, prev.value);
      debug_assert!(old.is_none());
    }
  }

  pub fn map<B: Eq, F: FnMut(ArchivedSnapshot<&T>) -> B>(&self, mut f: F) -> Archived<B> {
    let iter = self.iter().map(|s| {
      let time = s.start_time();
      let last_seen = s.last_seen();
      let value = f(s);
      (time, value, last_seen)
    });
    Archived::from_values(iter)
  }

  pub fn from_values<I: IntoIterator<Item = (Instant, T, Instant)>>(iter: I) -> Self {
    let mut iter = iter.into_iter().map(|(t, v, l)| (t, ArchivedValue::new(v, l)));
    let mut cur = iter.next().expect("Cannot construct Archived<T> from empty iterator");
    assert!(cur.0 <= cur.1.last_seen, "last_seen cannot be before start_time");

    let mut old: BTreeMap<Instant, ArchivedValue<T>> = BTreeMap::new();
    for next in iter {
      assert!(next.0 <= next.1.last_seen, "last_seen cannot be before start_time");
      assert!(
        next.1.last_seen > cur.0,
        "Values must be provided in cronological order"
      );

      if next.1.value == cur.1.value {
        cur.1.last_seen = next.1.last_seen;
      } else {
        let prev = core::mem::replace(&mut cur, next);
        old.insert(prev.0, prev.1);
      }
    }

    Archived(Temporal {
      current: SnapshotFrom::new(cur.0, cur.1),
      old,
    })
  }
}
