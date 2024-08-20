# Prerequisites

Sibyl needs an installed Oracle client in order to link either `OCI.DLL` on Windows or `libclntsh.so` on Linux. The minimal supported client is 12.2 as Sibyl uses some API functions that are not available in earlier clients.

> *Note* that while supporting 12.1 and older clients is definitely feasible, it was not a priority.

# Using Sibyl In A Project

Sibyl has 2 features - `blocking` and `nonblocking`. They are **exclusive** and **one** must be explicitly selected as neither is the default. Thus, when Sibyl is used as a dependency, it might be included as:

```toml
[dependencies]
sibyl = { version = "0.6", features = ["blocking"] }
```

A `nonblocking` mode also needs to know which async runtime/executor it is allowed to use to spawn async tasks. The async runtime selection is also controlled by a set of exclusive features. For now, Sibyl supports `tokio`, `actix`, `async-std`, and `async-global`. One of these must be specified with the `nonblocking` feature. For example:

```toml
[dependencies]
sibyl = { version = "0.6", features = ["nonblocking", "tokio"] }
```

# Building

The cargo build needs to know where the OCI client library is. You can provide that information via environment variable `OCI_LIB_DIR` on Windows or `LIBRARY_PATH` on Linux. On Linux, depending on which Oracle client is installed and how it was installed, the `LD_LIBRARY_PATH` might also be needed. `LIBRARY_PATH` (and `LD_LIBRARY_PATH`) would include the path to the directory with `libclntsh.so`. For example, you might build Sibyl's examples as:

```shell
export LIBRARY_PATH=/opt/instantclient_19_24
export LD_LIBRARY_PATH=/opt/instantclient_19_24
cargo build --examples --features=blocking
```

On Windows the process is similar if the target environment is `gnu`. The `OCI_LIB_DIR` would point to the directory with `oci.dll`:

```plaintext
set OCI_LIB_DIR=%ORACLE_HOME%\bin
cargo build --examples --features=blocking
```

> *Note* that for `gnu` targets the build script will try to locate `OCI.DLL` by searching it in the current `PATH` if the `OCI_LIB_DIR` is not specified.

However, for `msvc` environment the `OCI_LIB_DIR` must exist and point to the directory with `oci.lib`.  The build will fail if `OCI_LIB_DIR` is not set. For example, you might build those examples as:

```plaintext
set OCI_LIB_DIR=%ORACLE_HOME%\oci\lib\msvc
cargo build --examples --features=blocking
```

Because of the above requirement, that the `OCI_LIB_DIR` must be set for `msvc` targets, it also must be specified for the `rust-analyzer`. For example, in VS Code this can be done in `.vscode\settings.json`:

```json
"rust-analyzer.server.extraEnv": { "OCI_LIB_DIR": "C:\\Path\\To\\Oracle\\instantclient\\sdk\\lib\\msvc" }
```