// The mod's row in Options > Mods: the mod's identity and what it does.
// Copyright (c) 2026 hakergeniusz. MIT.
fn mlg_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "mlg".into(),
        title: "Auto-MLG 0.1.0".into(),
        description: "Places your water bucket under you in the last stretch of a fall — the server itself sees water, so no fall damage registers. Needs a water bucket in the hotbar; saves most falls on low-latency worlds and the slower ones elsewhere.".into(),
        keys: vec![],
    });
}
