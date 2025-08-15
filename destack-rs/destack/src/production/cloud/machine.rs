//! destack.production.cloud.machine@2025.08.15.1

#![destack::partial(destack.production.cloud.machine, file)]

#[destack::generated(MachineType, -, block)]
/// MachineType
pub enum MachineType {
    /// The main Destack runtime
    Runtime = 10,
    /// A Linux machine running Ubuntu
    Ubuntu = 1000,
    /// A Mac machine
    Mac = 1100,
    /// A Windows machine
    Windows = 1200,
    /// A custom Docker image
    Custom = 9000,
}

#[destack::generated(Region, -, block)]
/// Regions in an Area on a Continent.
pub enum Region {
    Zurich = 1000,
    Frankfurt = 1010,
    Virginia = 2000,
    Ohio = 2010,
    Oregon = 2200,
    SaoPaulo = 3000,
    CapeTown = 5000,
    Mumbai = 6000,
    Singapore = 6200,
    Tokyo = 6400,
    Sydney = 7000,
}

#[destack::generated(RegionArea, -, block)]
/// A larger Area of Regions within a Continent.
pub enum RegionArea {
    EuropeCentral = 1000,
    NorthAmericaEast = 2000,
    NorthAmericaWest = 2200,
    SouthAmericaEast = 3000,
    MiddleEastCentral = 4000,
    MiddleEastWest = 4200,
    AfricaSouth = 5000,
    AsiaWest = 6000,
    AsiaSouth = 6200,
    AsiaEast = 6400,
    AustraliaSouth = 7000,
}

#[destack::generated(RegionContinent, -, block)]
/// 'Continents' of Regions.
pub enum RegionContinent {
    Europe = 1000,
    NorthAmerica = 2000,
    SouthAmerica = 3000,
    MiddleEast = 4000,
    Africa = 5000,
    Asia = 6000,
    Australia = 7000,
}
