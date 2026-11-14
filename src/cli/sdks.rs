//! Resolve each selected SDK once; reuse its metadata for assembly and inventory markers.

use super::args::Args;
#[cfg(any(feature = "cuda", feature = "optix"))]
use super::args::SdkMode;
use vcv_rs::{Arch, Env, VcpkgInfo, discovery::RootRequest, env};

pub(super) struct Selection {
    #[cfg(feature = "cuda")]
    cuda: Option<vcv_rs::CudaInfo>,
    #[cfg(feature = "optix")]
    optix: Option<vcv_rs::OptixInfo>,
    #[cfg(feature = "nvsdk")]
    nvidia: Vec<vcv_rs::nvsdk::SdkInfo>,
    vcpkg: Option<VcpkgInfo>,
}

fn invalid_root(name: &str, request: &RootRequest) -> String {
    format!(
        "{name}: invalid explicit root {}={:?}",
        request.variable, request.path
    )
}

#[cfg(any(feature = "cuda", feature = "optix"))]
fn configured<T>(
    name: &str,
    mode: SdkMode,
    found: Option<T>,
    request: Option<RootRequest>,
    root_vars: &[&str],
) -> Result<Option<T>, String> {
    if found.is_none() {
        if let Some(request) = request {
            return Err(invalid_root(name, &request));
        }
        if mode == SdkMode::On {
            return Err(format!(
                "{name} not found; configure {}",
                root_vars.join(" / ")
            ));
        }
    }
    Ok(found)
}

pub(super) fn resolve(_args: &Args, target: Arch) -> Result<Selection, String> {
    #[cfg(feature = "cuda")]
    let cuda = match _args.cuda {
        SdkMode::Off => None,
        mode => configured(
            "CUDA Toolkit",
            mode,
            vcv_rs::cuda::detect_cuda(),
            vcv_rs::cuda::requested_root(),
            vcv_rs::cuda::root_vars(),
        )?,
    };
    #[cfg(feature = "optix")]
    let optix = match _args.optix {
        SdkMode::Off => None,
        mode => configured(
            "OptiX SDK",
            mode,
            vcv_rs::optix::detect_optix(),
            vcv_rs::optix::requested_root(),
            vcv_rs::optix::root_vars(),
        )?,
    };
    #[cfg(feature = "nvsdk")]
    let nvidia = {
        let mut found = Vec::new();
        if !_args.no_nvsdk {
            for kind in vcv_rs::nvsdk::Kind::ALL {
                let request = kind.requested_root();
                if !kind.is_supported() {
                    if let Some(request) = request {
                        return Err(format!(
                            "{} is unsupported on {}; {}={:?}",
                            kind.name(),
                            std::env::consts::OS,
                            request.variable,
                            request.path
                        ));
                    }
                    continue;
                }
                match vcv_rs::nvsdk::detect(kind, target) {
                    Some(sdk) => found.push(sdk),
                    None => {
                        if let Some(request) = request {
                            return Err(invalid_root(kind.name(), &request));
                        }
                    }
                }
            }
        }
        found
    };
    let request = env::requested_vcpkg_root();
    let triplet = env::requested_vcpkg_triplet();
    let vcpkg = match env::detect_vcpkg() {
        Some(root) => Some(env::probe_vcpkg(&root, target).ok_or_else(|| {
            format!("vcpkg at {root:?} has no supported installed triplet for {target}; requested triplet {triplet:?}")
        })?),
        None => {
            if let Some(request) = request {
                return Err(invalid_root("vcpkg", &request));
            }
            if triplet.is_some() {
                return Err(format!("vcpkg triplet {triplet:?} requires a configured VCPKG_ROOT"));
            }
            None
        }
    };
    Ok(Selection {
        #[cfg(feature = "cuda")]
        cuda,
        #[cfg(feature = "optix")]
        optix,
        #[cfg(feature = "nvsdk")]
        nvidia,
        vcpkg,
    })
}

impl Selection {
    pub(super) fn apply(&self, environment: &mut Env) -> Result<(), String> {
        #[cfg(feature = "cuda")]
        if let Some(cuda) = &self.cuda {
            env::add_cuda(environment, cuda).map_err(|error| error.to_string())?;
        }
        #[cfg(feature = "optix")]
        if let Some(optix) = &self.optix {
            env::add_optix(environment, optix);
        }
        #[cfg(feature = "nvsdk")]
        for sdk in &self.nvidia {
            env::add_nvsdk(environment, sdk).map_err(|error| error.to_string())?;
        }
        if let Some(vcpkg) = &self.vcpkg {
            env::add_vcpkg(environment, vcpkg).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(super) fn cuda_bounds(&self) -> Option<(Option<u16>, Option<u16>)> {
        #[cfg(feature = "cuda")]
        {
            self.cuda
                .as_ref()
                .and_then(|cuda| cuda.msvc)
                .map(|range| (range.min_vs_year(), range.max_vs_year()))
        }
        #[cfg(not(feature = "cuda"))]
        {
            None
        }
    }

    pub(super) fn info(&self, _target: Arch) -> Vec<String> {
        let mut info = Vec::new();
        #[cfg(feature = "cuda")]
        if let Some(cuda) = &self.cuda {
            info.push(format!(
                "CUDA {} | {} | link {} | runtime {}",
                cuda.version,
                cuda.root.display(),
                location(cuda.lib_dir(_target).as_deref()),
                location(cuda.runtime_dir(_target).as_deref())
            ));
            if let Some(range) = cuda.msvc {
                info.push(format!(
                    "CUDA header _MSC_VER range {}..{} (exclusive upper bound)",
                    range.min, range.max_exclusive
                ));
            }
        }
        #[cfg(feature = "optix")]
        if let Some(optix) = &self.optix {
            info.push(format!(
                "OptiX {} | ABI {} | headers {}",
                optix.version,
                optix.abi,
                optix.include_dir().display()
            ));
        }
        #[cfg(feature = "nvsdk")]
        for sdk in &self.nvidia {
            info.push(sdk_description(sdk));
        }
        if let Some(vcpkg) = &self.vcpkg {
            info.push(format!(
                "vcpkg {} | headers {} | link {} | runtime {}",
                vcpkg.triplet,
                vcpkg.include.display(),
                vcpkg.lib.display(),
                location(vcpkg.runtime_dir.as_deref())
            ));
        }
        info
    }
}

fn location(path: Option<&std::path::Path>) -> String {
    path.map_or_else(|| "none".into(), |path| path.display().to_string())
}

#[cfg(feature = "nvsdk")]
fn sdk_description(sdk: &vcv_rs::nvsdk::SdkInfo) -> String {
    format!(
        "{} {} | target {} | headers {} | link {} | runtime {}",
        sdk.kind.name(),
        sdk.version,
        sdk.target,
        sdk.include_dir.display(),
        location(sdk.lib_dir.as_deref()),
        location(sdk.runtime_dir.as_deref())
    )
}

pub(super) fn inventory(_args: &Args, selected: &Selection, _target: Arch) -> String {
    let mut output = String::new();
    #[cfg(feature = "cuda")]
    {
        let found = vcv_rs::cuda::list_toolkits();
        output.push_str(&format!(
            "\nCUDA ({} detected; mode {:?}):\n",
            found.len(),
            _args.cuda
        ));
        for cuda in found {
            let mark = if selected
                .cuda
                .as_ref()
                .is_some_and(|value| value.root == cuda.root)
            {
                "->"
            } else {
                "  "
            };
            output.push_str(&format!(
                "{mark} {} | {} | link {} | runtime {}\n",
                cuda.version,
                cuda.root.display(),
                location(cuda.lib_dir(_target).as_deref()),
                location(cuda.runtime_dir(_target).as_deref())
            ));
        }
    }
    #[cfg(feature = "optix")]
    {
        let found = vcv_rs::optix::list_sdks();
        output.push_str(&format!(
            "\nOptiX ({} detected; mode {:?}):\n",
            found.len(),
            _args.optix
        ));
        for optix in found {
            let mark = if selected
                .optix
                .as_ref()
                .is_some_and(|value| value.root == optix.root)
            {
                "->"
            } else {
                "  "
            };
            output.push_str(&format!(
                "{mark} {} | ABI {} | headers {}\n",
                optix.version,
                optix.abi,
                optix.include_dir().display()
            ));
        }
    }
    #[cfg(feature = "nvsdk")]
    {
        output.push_str("\nSeparately shipped NVIDIA SDKs:\n");
        for kind in vcv_rs::nvsdk::Kind::ALL {
            if !kind.is_supported() {
                output.push_str(&format!(
                    "  {}: unsupported on {}\n",
                    kind.name(),
                    std::env::consts::OS
                ));
                continue;
            }
            let found = vcv_rs::nvsdk::list(kind, _target);
            output.push_str(&format!(
                "  {}: {} detected{}\n",
                kind.name(),
                found.len(),
                if _args.no_nvsdk { "; disabled" } else { "" }
            ));
            for sdk in found {
                let mark = if selected.nvidia.iter().any(|value| {
                    value.root == sdk.root && value.kind == sdk.kind && value.target == sdk.target
                }) {
                    "->"
                } else {
                    "  "
                };
                output.push_str(&format!("{mark} {}\n", sdk_description(&sdk)));
            }
        }
    }
    match &selected.vcpkg {
        Some(vcpkg) => output.push_str(&format!(
            "\nvcpkg: {} | {} | link {} | runtime {}\n",
            vcpkg.triplet,
            vcpkg.root.display(),
            vcpkg.lib.display(),
            location(vcpkg.runtime_dir.as_deref())
        )),
        None => output.push_str("\nvcpkg: not configured\n"),
    }
    output
}

#[cfg(all(test, any(feature = "cuda", feature = "optix")))]
mod tests {
    use super::*;

    #[test]
    fn invalid_explicit_root_is_an_error_in_auto_and_on_modes() {
        for mode in [SdkMode::Auto, SdkMode::On] {
            let request = RootRequest {
                variable: "SDK_ROOT",
                path: "invalid-root".into(),
            };
            let error = configured::<()>("SDK", mode, None, Some(request), &["SDK_ROOT"])
                .expect_err("explicit invalid root");
            assert!(error.contains("invalid explicit root"));
        }
    }

    #[test]
    fn absence_and_selection_follow_the_requested_mode() {
        assert_eq!(
            configured::<()>("SDK", SdkMode::Auto, None, None, &["SDK_ROOT"]),
            Ok(None)
        );
        assert!(configured::<()>("SDK", SdkMode::On, None, None, &["SDK_ROOT"]).is_err());
        assert_eq!(
            configured("SDK", SdkMode::On, Some(7), None, &["SDK_ROOT"]),
            Ok(Some(7))
        );
    }
}
