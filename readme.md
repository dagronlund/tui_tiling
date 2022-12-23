## TUI Layout

TUI management framework for use with the TUI/Crossterm crates. Handles paneling and input direction to the appropriate component depending on focus. Each component is responsible for its own local state but does not need to handle switching focus to other components.