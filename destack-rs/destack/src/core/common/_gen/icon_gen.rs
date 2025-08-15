//! destack.core.common.icon@2025.08.15.1

#![destack::generated(destack.core.common.icon, file)]

use crate::Icon;
use crate::IconType;
use crate::PrimitiveType;
use crate::PropertyZone;
use crate::ReferenceType;
use crate::ScalarType;
use crate::TypeCardinality;
use crate::ValueFactory;

#[destack::generated(Icon, Debug, block)]
impl std::fmt::Debug for Icon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Icon")
    }
}

#[destack::generated(PrimitiveType, Debug, block)]
impl std::fmt::Debug for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::None => write!(f, "NONE"),
            PrimitiveType::Boolean => write!(f, "BOOLEAN"),
            PrimitiveType::Int8 => write!(f, "INT8"),
            PrimitiveType::Int16 => write!(f, "INT16"),
            PrimitiveType::Int32 => write!(f, "INT32"),
            PrimitiveType::Int64 => write!(f, "INT64"),
            PrimitiveType::Int128 => write!(f, "INT128"),
            PrimitiveType::Uint8 => write!(f, "UINT8"),
            PrimitiveType::Uint16 => write!(f, "UINT16"),
            PrimitiveType::Uint32 => write!(f, "UINT32"),
            PrimitiveType::Uint64 => write!(f, "UINT64"),
            PrimitiveType::Uint128 => write!(f, "UINT128"),
            PrimitiveType::Float32 => write!(f, "FLOAT32"),
            PrimitiveType::Float64 => write!(f, "FLOAT64"),
            PrimitiveType::Datetime => write!(f, "DATETIME"),
            PrimitiveType::Date => write!(f, "DATE"),
            PrimitiveType::Time => write!(f, "TIME"),
            PrimitiveType::Timestamp => write!(f, "TIMESTAMP"),
            PrimitiveType::Duration => write!(f, "DURATION"),
            PrimitiveType::String => write!(f, "STRING"),
            PrimitiveType::Character => write!(f, "CHARACTER"),
            PrimitiveType::Uuid => write!(f, "UUID"),
            PrimitiveType::Bytes => write!(f, "BYTES"),
            PrimitiveType::Json => write!(f, "JSON"),
        }
    }
}

#[destack::generated(TypeCardinality, Debug, block)]
impl std::fmt::Debug for TypeCardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeCardinality::Scalar => write!(f, "SCALAR"),
            TypeCardinality::List => write!(f, "LIST"),
            TypeCardinality::Tuple => write!(f, "TUPLE"),
            TypeCardinality::Map => write!(f, "MAP"),
        }
    }
}

#[destack::generated(ScalarType, Debug, block)]
impl std::fmt::Debug for ScalarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalarType::Primitive => write!(f, "PRIMITIVE"),
            ScalarType::Enum => write!(f, "ENUM"),
            ScalarType::Node => write!(f, "NODE"),
            ScalarType::NodeRaw => write!(f, "NODE_RAW"),
            ScalarType::NodeIdentity => write!(f, "NODE_IDENTITY"),
            ScalarType::NodeSpatial => write!(f, "NODE_SPATIAL"),
            ScalarType::NodeTemporal => write!(f, "NODE_TEMPORAL"),
            ScalarType::Struct => write!(f, "STRUCT"),
            ScalarType::Handle => write!(f, "HANDLE"),
            ScalarType::Union => write!(f, "UNION"),
        }
    }
}

#[destack::generated(ValueFactory, Debug, block)]
impl std::fmt::Debug for ValueFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueFactory::Uuid7 => write!(f, "UUID7"),
            ValueFactory::Now => write!(f, "NOW"),
            ValueFactory::RemoteEpoch => write!(f, "REMOTE_EPOCH"),
            ValueFactory::LocalEpoch => write!(f, "LOCAL_EPOCH"),
            ValueFactory::Actor => write!(f, "ACTOR"),
            ValueFactory::Client => write!(f, "CLIENT"),
            ValueFactory::ClientNonce => write!(f, "CLIENT_NONCE"),
            ValueFactory::Region => write!(f, "REGION"),
            ValueFactory::SelfNode => write!(f, "SELF_NODE"),
            ValueFactory::CurrentSpace => write!(f, "CURRENT_SPACE"),
            ValueFactory::CurrentBranch => write!(f, "CURRENT_BRANCH"),
            ValueFactory::CurrentSnapshot => write!(f, "CURRENT_SNAPSHOT"),
            ValueFactory::Name => write!(f, "NAME"),
        }
    }
}

#[destack::generated(PropertyZone, Debug, block)]
impl std::fmt::Debug for PropertyZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertyZone::Member => write!(f, "MEMBER"),
            PropertyZone::Input => write!(f, "INPUT"),
            PropertyZone::Output => write!(f, "OUTPUT"),
        }
    }
}

#[destack::generated(ReferenceType, Debug, block)]
impl std::fmt::Debug for ReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceType::Raw => write!(f, "RAW"),
            ReferenceType::Identity => write!(f, "IDENTITY"),
            ReferenceType::Spatial => write!(f, "SPATIAL"),
            ReferenceType::Temporal => write!(f, "TEMPORAL"),
        }
    }
}

#[destack::generated(IconType, Debug, block)]
impl std::fmt::Debug for IconType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconType::Emoji => write!(f, "EMOJI"),
            IconType::File => write!(f, "FILE"),
            IconType::FileUrl => write!(f, "FILE_URL"),
        }
    }
}
