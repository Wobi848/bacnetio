//! Recording values: chosen points read at a fixed interval and kept in memory, for a chart and
//! CSV (concept section 4, "Later": record values).

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use bacnet_types::enums::EngineeringUnits;
use bacnet_types::primitives::{ObjectIdentifier, PropertyValue};

/// Points in one recording; each costs one request per interval.
pub const MAX_POINTS: usize = 32;
/// Samples kept per point; the oldest are dropped. A week at 10 s is about 60 000.
pub const MAX_SAMPLES: usize = 100_000;
pub const MIN_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedPoint {
    pub device: u32,
    pub object: ObjectIdentifier,
    pub name: Option<String>,
    pub units: Option<EngineeringUnits>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub time: SystemTime,
    /// `None` when the device did not answer or the value is not a number.
    pub value: Option<f64>,
}

/// How a point is measured.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Read at the recording's interval.
    #[default]
    Poll,
    /// Every report the device sends, with its own time.
    Cov,
}

#[derive(Debug)]
pub struct Series {
    pub point: RecordedPoint,
    pub samples: VecDeque<Sample>,
    pub mode: Mode,
    /// The device refused to report this point; it is read at the interval instead.
    pub cov_refused: bool,
}

#[derive(Debug, Default)]
pub struct Recording {
    series: Vec<Series>,
}

impl Recording {
    pub fn add(&mut self, point: RecordedPoint) -> Result<(), String> {
        if self.position(point.device, point.object).is_some() {
            return Ok(());
        }
        if self.series.len() >= MAX_POINTS {
            return Err(format!("a recording holds at most {MAX_POINTS} points"));
        }
        self.series.push(Series {
            point,
            samples: VecDeque::new(),
            mode: Mode::default(),
            cov_refused: false,
        });
        Ok(())
    }

    /// False if the point was not recorded.
    pub fn remove(&mut self, device: u32, object: ObjectIdentifier) -> bool {
        match self.position(device, object) {
            Some(index) => {
                self.series.remove(index);
                true
            }
            None => false,
        }
    }

    pub fn series(&self) -> &[Series] {
        &self.series
    }

    pub fn points(&self) -> Vec<RecordedPoint> {
        self.series
            .iter()
            .map(|series| series.point.clone())
            .collect()
    }

    /// The points with how each is measured now: a refused COV counts as polling.
    pub fn points_with_mode(&self) -> Vec<(RecordedPoint, Mode)> {
        self.series
            .iter()
            .map(|series| {
                let mode = if series.cov_refused {
                    Mode::Poll
                } else {
                    series.mode
                };
                (series.point.clone(), mode)
            })
            .collect()
    }

    /// False if the point is not recorded.
    pub fn set_mode(&mut self, device: u32, object: ObjectIdentifier, mode: Mode) -> bool {
        let Some(index) = self.position(device, object) else {
            return false;
        };
        let series = &mut self.series[index];
        series.mode = mode;
        series.cov_refused = false;
        true
    }

    pub fn cov_refused(&mut self, device: u32, object: ObjectIdentifier) {
        if let Some(index) = self.position(device, object) {
            self.series[index].cov_refused = true;
        }
    }

    /// Stores a device's report, for a point recorded by COV.
    pub fn record_report(
        &mut self,
        device: u32,
        object: ObjectIdentifier,
        time: SystemTime,
        value: Option<f64>,
    ) {
        let Some(index) = self.position(device, object) else {
            return;
        };
        if self.series[index].mode != Mode::Cov || self.series[index].cov_refused {
            return;
        }
        let point = self.series[index].point.clone();
        self.record(time, &[(point, value)]);
    }

    /// Stores the values of one round. Points removed meanwhile are skipped.
    pub fn record(&mut self, time: SystemTime, values: &[(RecordedPoint, Option<f64>)]) {
        for (point, value) in values {
            let Some(index) = self.position(point.device, point.object) else {
                continue;
            };
            let samples = &mut self.series[index].samples;
            if samples.len() >= MAX_SAMPLES {
                samples.pop_front();
            }
            samples.push_back(Sample {
                time,
                value: *value,
            });
        }
    }

    pub fn clear_samples(&mut self) {
        for series in &mut self.series {
            series.samples.clear();
        }
    }

    fn position(&self, device: u32, object: ObjectIdentifier) -> Option<usize> {
        self.series
            .iter()
            .position(|series| series.point.device == device && series.point.object == object)
    }
}

/// A present value as a number for the chart: binary values as 0 and 1, states as their number.
pub fn number(value: &PropertyValue) -> Option<f64> {
    match value {
        // through the shortest decimal, so 21.4 stays 21.4 and does not become 21.399999618530273
        PropertyValue::Real(value) => value.to_string().parse().ok(),
        PropertyValue::Double(value) => Some(*value),
        PropertyValue::Unsigned(value) => Some(*value as f64),
        PropertyValue::Signed(value) => Some(f64::from(*value)),
        PropertyValue::Enumerated(value) => Some(f64::from(*value)),
        PropertyValue::Boolean(value) => Some(f64::from(u8::from(*value))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bacnet_types::enums::ObjectType;

    fn point(instance: u32) -> RecordedPoint {
        RecordedPoint {
            device: 1,
            object: ObjectIdentifier::new(ObjectType::ANALOG_INPUT, instance).unwrap(),
            name: None,
            units: None,
        }
    }

    #[test]
    fn reports_are_kept_only_for_points_recorded_by_cov() {
        let mut recording = Recording::default();
        recording.add(point(1)).unwrap();
        let object = point(1).object;
        recording.record_report(1, object, SystemTime::UNIX_EPOCH, Some(1.0));
        assert!(
            recording.series()[0].samples.is_empty(),
            "polled points ignore reports"
        );
        assert!(recording.set_mode(1, object, Mode::Cov));
        recording.record_report(1, object, SystemTime::UNIX_EPOCH, Some(2.0));
        assert_eq!(recording.series()[0].samples.len(), 1);
        recording.cov_refused(1, object);
        assert_eq!(
            recording.points_with_mode()[0].1,
            Mode::Poll,
            "refused: polled instead"
        );
        recording.record_report(1, object, SystemTime::UNIX_EPOCH, Some(3.0));
        assert_eq!(recording.series()[0].samples.len(), 1);
    }

    #[test]
    fn points_are_added_once_and_removed() {
        let mut recording = Recording::default();
        recording.add(point(1)).unwrap();
        recording.add(point(1)).unwrap();
        recording.add(point(2)).unwrap();
        assert_eq!(recording.points().len(), 2);
        assert!(recording.remove(1, point(1).object));
        assert!(!recording.remove(1, point(1).object));
        assert_eq!(recording.points(), vec![point(2)]);
    }

    #[test]
    fn too_many_points_are_refused() {
        let mut recording = Recording::default();
        for instance in 0..MAX_POINTS as u32 {
            recording.add(point(instance)).unwrap();
        }
        assert!(recording.add(point(999)).is_err());
    }

    #[test]
    fn samples_of_removed_points_are_skipped() {
        let mut recording = Recording::default();
        recording.add(point(1)).unwrap();
        let time = SystemTime::UNIX_EPOCH;
        recording.record(time, &[(point(1), Some(21.5)), (point(2), Some(3.0))]);
        assert_eq!(recording.series().len(), 1);
        assert_eq!(
            recording.series()[0].samples[0],
            Sample {
                time,
                value: Some(21.5)
            }
        );
        recording.clear_samples();
        assert!(recording.series()[0].samples.is_empty());
    }

    #[test]
    fn values_become_numbers() {
        assert_eq!(number(&PropertyValue::Real(1.5)), Some(1.5));
        assert_eq!(number(&PropertyValue::Real(21.4)), Some(21.4));
        assert_eq!(number(&PropertyValue::Enumerated(1)), Some(1.0));
        assert_eq!(number(&PropertyValue::Unsigned(3)), Some(3.0));
        assert_eq!(number(&PropertyValue::Null), None);
    }
}
