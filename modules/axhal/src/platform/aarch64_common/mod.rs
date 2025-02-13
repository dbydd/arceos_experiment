mod boot;

pub mod generic_timer;
#[cfg(not(any(
    all(platform_family = "aarch64-raspi"),
    all(platform_family = "aarch64-phytium-pi"),
)))]
pub mod psci;

#[cfg(feature = "irq")]
pub mod gic;

#[cfg(not(platform_family = "aarch64-bsta1000b"))]
pub mod pl011;
