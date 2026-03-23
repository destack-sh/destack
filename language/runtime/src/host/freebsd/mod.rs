#[cfg(any(test, target_os = "freebsd"))]
pub(crate) mod abi;
#[cfg(target_os = "freebsd")]
mod adapter;
#[cfg(any(test, target_os = "freebsd"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "freebsd")]
pub(crate) use adapter::FreeBsdHost;
