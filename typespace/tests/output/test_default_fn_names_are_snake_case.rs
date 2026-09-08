#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub enum DensityDistribution {
    NormalDist {
        #[serde(default = "defaults::density_distribution_normal_dist_stdev")]
        stdev: f64,
    },
    UniformDist {
        #[serde(default = "defaults::density_distribution_uniform_dist_max_value")]
        max_value: f64,
    },
}
#[derive(::serde::Deserialize, ::serde::Serialize, Debug, PartialEq)]
pub struct ForceTransform {
    #[serde(default = "defaults::force_transform_alpha_min")]
    pub alpha_min: f64,
}
/// Generation of default values for serde.
pub mod defaults {
    pub(super) fn density_distribution_normal_dist_stdev() -> f64 {
        1.5_f64
    }
    pub(super) fn density_distribution_uniform_dist_max_value() -> f64 {
        2.5_f64
    }
    pub(super) fn force_transform_alpha_min() -> f64 {
        0.5_f64
    }
}
