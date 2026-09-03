# Tasks

## 1. Tokens

- [x] 1.1 Set every radius token to zero and every shadow token to none, keeping the declarations
- [x] 1.2 Retune the light and dark palettes to the values in the design and add the corner tick token
- [x] 1.3 Record measured contrast ratios for every text and hairline pair in both themes (table recorded in design.md)

## 2. Primitives

- [x] 2.1 Restyle header, kicker, headings, labels, and buttons
- [x] 2.2 Restyle the status strip, chips, and counters with tone glyphs
- [x] 2.3 Add corner ticks to cards and the message workspace, and remove elevation
- [x] 2.4 Restyle the destination bar and its active treatment
- [x] 2.5 Restyle the scan indicator, banners, and the canvas texture

## 3. Evidence

- [x] 3.1 Update the stylesheet contract test for the new invariants (radius-zero, compact-thread, and direction selectors added; timestamp and propagation-hint assertions updated)
- [x] 3.2 Run the styrene-ui-app tests and the workspace validation set (`cargo test -p styrene-ui-app`, 67 passed)
- [x] 3.3 Capture every tab in light and dark on the simulator and retain the captures (`target/ios-xcuitest/review-{light,dark}/`, tabs plus the open thread and the largest-text set)
- [x] 3.4 Run the packaged XCUI suite in both appearances (17 tests, 0 failures, light and dark)

## 4. Thread screen

- [x] 4.1 Make the open thread its own compact screen with an internal-scrolling history and a one-line composer
- [x] 4.2 Mark message direction by rule side and offset, and render timestamps as UTC date-time groups
- [x] 4.3 Show one composer status line and only mention propagated delivery when selected or unavailable
- [x] 4.4 Capture the open thread in the review suite at the default text size
