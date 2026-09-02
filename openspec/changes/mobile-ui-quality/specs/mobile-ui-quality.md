# Mobile UI Quality - Delta Spec

## ADDED Requirements

### Requirement: Each screen has one title and a compact status strip

The mobile shell must render each screen name once, place session status in a
single strip with trailing safe-area padding, and keep the header and strip
within a quarter of the viewport height on the reference device.

#### Scenario: Tab renders its content near the top
Given the fixture build on an iPhone 17 Pro viewport
When any primary tab is shown
Then the screen name appears exactly once
And the first content card begins within the top quarter of the viewport

### Requirement: Status tones are distinct and reserved

Positive, caution, negative, and neutral tones must each have a distinct
rendering in both appearances, counters and labels must not use the positive
rendering, and the accent colour must be used only for enabled primary actions
and unread indicators.

#### Scenario: Connected session next to a counter
Given a connected session and an inbox count of zero
When the Messages tab is shown
Then the session chip uses the positive rendering
And the counter uses the neutral rendering

#### Scenario: Disabled primary action
Given an action that is unavailable in the current view
When its control is rendered
Then it uses the shared disabled treatment
And it does not use the accent or danger fill

### Requirement: Prose never breaks inside a word

Only identifier text may wrap at arbitrary characters. Headings, labels, and
body text wrap at word boundaries and use a type scale sized for the mobile
viewport.

#### Scenario: Long heading at narrow width
Given the Messages tab at the reference viewport width
When the inbox heading is rendered
Then it wraps between words or fits on one line
And no character-level break occurs

### Requirement: Primary navigation uses icons

The tab bar must render an icon with each label and keep each control at least
44 by 44 points.

#### Scenario: Tab bar at default text size
Given the shell at the default Dynamic Type size
When the tab bar is rendered
Then every tab shows an icon and a label
And the minimum-size test still passes

### Requirement: RNode reconnection is explicit and never self-cancelled

The application must not initiate a Bluetooth connection at launch. Reconnection
to a remembered RNode is an operator action with a visible cancel, and a
connection that CoreBluetooth has reported as connected is not cancelled by an
application deadline.

#### Scenario: Launch with a remembered RNode
Given an approved RNode was stored in an earlier session
When the application launches
Then no connection is attempted
And the Network tab offers a reconnect action for that RNode

#### Scenario: Pairing request during reconnection
Given the operator chose to reconnect and iOS shows a pairing request
When the request stays open longer than the connect deadline
Then the application does not cancel the connection
And the pairing request remains until the operator answers it
