# ArenaBuddy UI - Dioxus Frontend

This is the frontend UI for ArenaBuddy, converted from Leptos to Dioxus.

## Conversion Summary

This codebase was originally built with Leptos and has been converted to use Dioxus as the frontend framework. The conversion maintains the same functionality while adapting to Dioxus's component model and patterns.

## Key Changes Made

### Framework Migration
- **Leptos → Dioxus**: Complete migration from Leptos v0.8 to Dioxus v0.7.0-alpha.3
- **Router**: Migrated from `leptos_router` to `dioxus-router`
- **Component Syntax**: Updated from Leptos `view!` macro to Dioxus `rsx!` macro

### Component Updates
All components were converted to use Dioxus patterns:

- **App Component**: Updated router structure and navigation
- **Matches**: Match list display and loading states
- **MatchDetails**: Individual match view with deck lists and mulligan info
- **MatchInfo**: Player information display
- **DeckList**: Card deck visualization with mana costs
- **MulliganDisplay**: Mulligan decision visualization
- **ErrorLogs**: Error log viewer
- **DebugLogs**: Debug log configuration
- **ManaCost**: Mana symbol display component

### State Management
- Replaced Leptos signals with Dioxus `use_signal`
- Updated reactive patterns for Dioxus lifecycle
- Converted `spawn_local` to `spawn` for async operations
- Updated effect handling with `use_effect`

### Type System Updates
Added `PartialEq` derives to core types to satisfy Dioxus component requirements:
- `Cost` and `CostSymbol` (mana system)
- `MTGAMatch` (match data)
- `DeckDisplayRecord` (deck display)
- `Color` enum (mana colors)

### Routing
- Implemented nested routing with layout components
- Updated navigation links to use Dioxus `Link` components
- Converted route parameters handling

## Building and Running

### Prerequisites
- Rust 1.88+
- wasm-pack (for web builds)

### Development
```bash
# Check the code
cargo check --manifest-path arenabuddy/arenabuddy_ui/Cargo.toml

# Build for web
cargo build --manifest-path arenabuddy/arenabuddy_ui/Cargo.toml --target wasm32-unknown-unknown

# Or use dx for development (if available)
dx serve --hot-reload
```

## Dependencies

- `dioxus`: Main framework (v0.7.0-alpha.3)
- `dioxus-router`: Routing (v0.7.0-alpha.3)
- `console_error_panic_hook`: Better error messages in WASM
- `wasm-bindgen`: WASM bindings
- `serde` & `serde-wasm-bindgen`: Serialization for JS interop

## Features

- **Match History**: View and browse MTG Arena match history
- **Match Details**: Detailed view of individual matches including:
  - Player information
  - Deck composition with mana costs
  - Mulligan decisions with card images
- **Error Management**: View application error logs
- **Debug Configuration**: Configure debug log directory
- **Responsive Design**: Mobile-friendly interface with Tailwind CSS

## Architecture

The UI follows a component-based architecture:

```
src/
├── app.rs              # Main app and routing
├── main.rs             # Entry point
├── state.rs            # Shared state types
├── matches.rs          # Match list component
├── match_details.rs    # Individual match view
├── error_logs.rs       # Error log viewer
├── debug_logs.rs       # Debug configuration
└── components/
    ├── mod.rs          # Component exports
    ├── cost.rs         # Mana cost display
    ├── deck_list.rs    # Deck composition view
    ├── match_info.rs   # Match metadata display
    └── mulligan_display.rs  # Mulligan visualization
```

## Integration with Tauri

This frontend is designed to work with the Tauri backend, using `invoke` calls to:
- Fetch match data
- Retrieve logs
- Configure settings
- Open external URLs

## Development Notes

- The conversion maintains the same UI/UX as the original Leptos version
- All TypeScript/JavaScript interop is preserved
- Tailwind CSS classes remain unchanged
- Component props now use Dioxus's `Properties` trait system
- Async operations use Dioxus's `spawn` instead of `spawn_local`

## Known Limitations

- Some advanced Leptos features may not have direct Dioxus equivalents
- Image error handling (onerror) was simplified during conversion
- Component memoization patterns differ between frameworks

This conversion provides a solid foundation for further development with Dioxus while maintaining all the original functionality.