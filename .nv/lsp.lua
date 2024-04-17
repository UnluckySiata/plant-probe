return {
    rust_analyzer = {
        ["rust-analyzer"] = {
            check = {
                allTargets = false,
            },
            cargo = {
                target = "thumbv6m-none-eabi",
            },
        }
    }
}
