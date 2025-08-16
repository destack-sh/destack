//! destack.production.cloud.machine@2025.08.15.1

#![destack::generated(destack.production.cloud.machine, file)]

use crate::{MachineType, Region, RegionArea, RegionContinent};

#[destack::generated(MachineType, Debug, block)]
impl std::fmt::Debug for MachineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineType::Runtime => write!(f, "RUNTIME"),
            MachineType::Ubuntu => write!(f, "UBUNTU"),
            MachineType::Mac => write!(f, "MAC"),
            MachineType::Windows => write!(f, "WINDOWS"),
            MachineType::Custom => write!(f, "CUSTOM"),
        }
    }
}

#[destack::generated(Region, Debug, block)]
impl std::fmt::Debug for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Zurich => write!(f, "ZURICH"),
            Region::Frankfurt => write!(f, "FRANKFURT"),
            Region::Virginia => write!(f, "VIRGINIA"),
            Region::Ohio => write!(f, "OHIO"),
            Region::Oregon => write!(f, "OREGON"),
            Region::SaoPaulo => write!(f, "SAO_PAULO"),
            Region::CapeTown => write!(f, "CAPE_TOWN"),
            Region::Mumbai => write!(f, "MUMBAI"),
            Region::Singapore => write!(f, "SINGAPORE"),
            Region::Tokyo => write!(f, "TOKYO"),
            Region::Sydney => write!(f, "SYDNEY"),
        }
    }
}

#[destack::generated(RegionArea, Debug, block)]
impl std::fmt::Debug for RegionArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionArea::EuropeCentral => write!(f, "EUROPE_CENTRAL"),
            RegionArea::NorthAmericaEast => write!(f, "NORTH_AMERICA_EAST"),
            RegionArea::NorthAmericaWest => write!(f, "NORTH_AMERICA_WEST"),
            RegionArea::SouthAmericaEast => write!(f, "SOUTH_AMERICA_EAST"),
            RegionArea::MiddleEastCentral => write!(f, "MIDDLE_EAST_CENTRAL"),
            RegionArea::MiddleEastWest => write!(f, "MIDDLE_EAST_WEST"),
            RegionArea::AfricaSouth => write!(f, "AFRICA_SOUTH"),
            RegionArea::AsiaWest => write!(f, "ASIA_WEST"),
            RegionArea::AsiaSouth => write!(f, "ASIA_SOUTH"),
            RegionArea::AsiaEast => write!(f, "ASIA_EAST"),
            RegionArea::AustraliaSouth => write!(f, "AUSTRALIA_SOUTH"),
        }
    }
}

#[destack::generated(RegionContinent, Debug, block)]
impl std::fmt::Debug for RegionContinent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionContinent::Europe => write!(f, "EUROPE"),
            RegionContinent::NorthAmerica => write!(f, "NORTH_AMERICA"),
            RegionContinent::SouthAmerica => write!(f, "SOUTH_AMERICA"),
            RegionContinent::MiddleEast => write!(f, "MIDDLE_EAST"),
            RegionContinent::Africa => write!(f, "AFRICA"),
            RegionContinent::Asia => write!(f, "ASIA"),
            RegionContinent::Australia => write!(f, "AUSTRALIA"),
        }
    }
}
