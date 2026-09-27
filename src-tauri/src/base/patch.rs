//! Helpers for partial updates sent by the front-end.

use serde::{Deserialize, Deserializer};

/// For `Option<Option<T>>` patch fields (with `#[serde(default)]` on the
/// struct): tells "sent as null" (`Some(None)`: clear the value) from
/// "absent" (`None`: keep it). Without it serde maps both to `None`.
pub fn nullable<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}
