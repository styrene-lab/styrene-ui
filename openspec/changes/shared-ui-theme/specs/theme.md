# Shared theme — Delta Spec

## ADDED Requirements

### Requirement: Shared visual roles preserve shell layouts
Desktop and mobile use the same palette, typography, and shape tokens without sharing shell layout.

#### Scenario: System appearance changes
Given either application is open
When the operating system switches appearance
Then the application uses the matching shared light or dark palette
And its navigation and pane geometry remain unchanged

### Requirement: Operational meaning survives theme changes
Status roles remain distinct from selection and both palettes provide readable text.

#### Scenario: Palette validation
Given the shared Field Kit preset
When its foreground and surface pairs are validated
Then normal text pairs meet a contrast ratio of at least 4.5
And status roles do not derive from chart colors

### Requirement: Published theme values remain canonical
The shared preset preserves both published palettes and common variables. Font selection works offline. Application contrast adjustments are separate semantic aliases.

#### Scenario: Published Field Kit theme
Given the registry snapshot for cmtr8h1vu000204i6gaao9ld2
When the desktop or mobile shell renders
Then its preset contains every published common and appearance-specific value
And selected font data is available without network requests
