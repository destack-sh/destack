#[cfg(any(test, target_os = "illumos"))]
pub(crate) mod abi;
#[cfg(target_os = "illumos")]
mod adapter;
#[cfg(any(test, target_os = "illumos"))]
mod ingress;
#[cfg(test)]
mod tests;

#[cfg(target_os = "illumos")]
pub(crate) use adapter::IllumosHost;
