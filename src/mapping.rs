//! The mapping table: which VSS signal backs which AAOS property, and how its unit converts.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::pb::kuksa::value::TypedValue;
use crate::pb::vhal;

/// One file of [`Property`] rows.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    pub properties: Vec<Property>,
}

/// How an AAOS property carries its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Float,
    Int32,
    Int64,
}

/// How an AAOS property changes, as `VehiclePropertyChangeMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeMode {
    Static,
    OnChange,
    Continuous,
}

/// One AAOS property and area, backed by one VSS signal. Read-only in 0.1.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Property {
    /// The AAOS name, for logs and `Dump`.
    pub name: String,
    /// The AAOS property ID, as in `VehicleProperty.aidl`.
    pub prop: i32,
    /// The area ID; 0 for a global property.
    #[serde(default)]
    pub area: i32,
    /// The VSS path in the databroker.
    pub vss: String,
    #[serde(rename = "type")]
    pub value_type: ValueType,
    /// AAOS value = VSS value * scale + offset.
    #[serde(default = "one")]
    pub scale: f64,
    #[serde(default)]
    pub offset: f64,
    pub change_mode: ChangeMode,
    #[serde(default)]
    pub min_sample_rate: f32,
    #[serde(default)]
    pub max_sample_rate: f32,
}

fn one() -> f64 {
    1.0
}

/// Why a mapping file was refused.
#[derive(Debug, thiserror::Error)]
pub enum MappingError {
    #[error("cannot read mapping {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("mapping {path} is not valid: {source}")]
    Parse {
        path: String,
        source: serde_yaml_ng::Error,
    },
    #[error("mapping has two rows for property {prop:#x} area {area}")]
    Duplicate { prop: i32, area: i32 },
    #[error(
        "mapping row {name}: a continuous property needs 0 < min_sample_rate <= max_sample_rate"
    )]
    SampleRate { name: String },
}

impl Mapping {
    pub fn load(path: &Path) -> Result<Self, MappingError> {
        let text = std::fs::read_to_string(path).map_err(|source| MappingError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text).map_err(|e| match e {
            MappingError::Parse { source, .. } => MappingError::Parse {
                path: path.display().to_string(),
                source,
            },
            other => other,
        })
    }

    pub fn parse(text: &str) -> Result<Self, MappingError> {
        let m: Mapping = serde_yaml_ng::from_str(text).map_err(|source| MappingError::Parse {
            path: "<text>".into(),
            source,
        })?;
        let mut seen = std::collections::HashSet::new();
        for p in &m.properties {
            if !seen.insert((p.prop, p.area)) {
                return Err(MappingError::Duplicate {
                    prop: p.prop,
                    area: p.area,
                });
            }
            if p.change_mode == ChangeMode::Continuous
                && !(p.min_sample_rate > 0.0 && p.min_sample_rate <= p.max_sample_rate)
            {
                return Err(MappingError::SampleRate {
                    name: p.name.clone(),
                });
            }
        }
        Ok(m)
    }

    /// Rows by `(prop, area)`.
    pub fn index(&self) -> HashMap<(i32, i32), Property> {
        self.properties
            .iter()
            .map(|p| ((p.prop, p.area), p.clone()))
            .collect()
    }
}

impl Property {
    /// The `VehiclePropConfig` AAOS reads at boot.
    pub fn config(&self) -> vhal::VehiclePropConfig {
        let change_mode = match self.change_mode {
            ChangeMode::Static => vhal::VehiclePropertyChangeMode::Static,
            ChangeMode::OnChange => vhal::VehiclePropertyChangeMode::OnChange,
            ChangeMode::Continuous => vhal::VehiclePropertyChangeMode::Continuous,
        };
        vhal::VehiclePropConfig {
            prop: self.prop,
            access: vhal::VehiclePropertyAccess::Read as i32,
            change_mode: change_mode as i32,
            area_configs: vec![vhal::VehicleAreaConfig {
                area_id: self.area,
                access: vhal::VehiclePropertyAccess::Read as i32,
                ..Default::default()
            }],
            min_sample_rate: self.min_sample_rate,
            max_sample_rate: self.max_sample_rate,
            ..Default::default()
        }
    }

    /// The AAOS value for a VSS value, or `None` when the VSS value is not a number.
    pub fn convert(&self, value: &TypedValue, timestamp_ns: i64) -> Option<vhal::VehiclePropValue> {
        let x = match value {
            TypedValue::Float(v) => f64::from(*v),
            TypedValue::Double(v) => *v,
            TypedValue::Int32(v) => f64::from(*v),
            TypedValue::Int64(v) => *v as f64,
            TypedValue::Uint32(v) => f64::from(*v),
            TypedValue::Uint64(v) => *v as f64,
            _ => return None,
        };
        let y = x * self.scale + self.offset;
        let mut out = vhal::VehiclePropValue {
            timestamp: timestamp_ns,
            area_id: self.area,
            prop: self.prop,
            status: vhal::VehiclePropertyStatus::Available as i32,
            ..Default::default()
        };
        match self.value_type {
            ValueType::Float => out.float_values = vec![y as f32],
            ValueType::Int32 => out.int32_values = vec![y.round() as i32],
            ValueType::Int64 => out.int64_values = vec![y.round() as i64],
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_mapping() -> Mapping {
        Mapping::parse(include_str!("../mappings/default.yaml")).unwrap()
    }

    #[test]
    fn the_default_mapping_parses_with_hex_ids() {
        let m = default_mapping();
        let speed = &m.properties[0];
        assert_eq!(speed.name, "PERF_VEHICLE_SPEED");
        assert_eq!(speed.prop, 0x1160_0207);
        assert_eq!(speed.area, 0);
    }

    #[test]
    fn vehicle_speed_converts_from_km_per_h_to_m_per_s() {
        let m = default_mapping();
        let v = m.properties[0]
            .convert(&TypedValue::Float(72.0), 7)
            .unwrap();
        assert_eq!(v.prop, 0x1160_0207);
        assert_eq!(v.timestamp, 7);
        assert!(
            (v.float_values[0] - 20.0).abs() < 1e-4,
            "{:?}",
            v.float_values
        );
    }

    #[test]
    fn an_integer_vss_value_fills_a_float_property() {
        let m = default_mapping();
        let rpm = m
            .properties
            .iter()
            .find(|p| p.name == "ENGINE_RPM")
            .unwrap();
        let v = rpm.convert(&TypedValue::Uint32(3000), 0).unwrap();
        assert_eq!(v.float_values, vec![3000.0]);
    }

    #[test]
    fn a_string_vss_value_does_not_convert() {
        let m = default_mapping();
        assert!(
            m.properties[0]
                .convert(&TypedValue::String("fast".into()), 0)
                .is_none()
        );
    }

    #[test]
    fn duplicate_rows_are_refused() {
        let text = "properties:\n  - {name: A, prop: 1, vss: X, type: float, change_mode: on_change}\n  - {name: B, prop: 1, vss: Y, type: float, change_mode: on_change}\n";
        assert!(matches!(
            Mapping::parse(text),
            Err(MappingError::Duplicate { prop: 1, area: 0 })
        ));
    }

    #[test]
    fn a_continuous_row_needs_sample_rates() {
        let text =
            "properties:\n  - {name: A, prop: 1, vss: X, type: float, change_mode: continuous}\n";
        assert!(matches!(
            Mapping::parse(text),
            Err(MappingError::SampleRate { .. })
        ));
    }
}
