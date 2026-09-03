# Mobile Panel Pass - Delta Spec

## ADDED Requirements

### Requirement: The propagation panel is a board

The propagation panel must show a readiness chip in its heading row, place
node selection and the synchronization action on one line, and keep its
policy and evidence sentences behind a disclosure while retaining every
element identifier.

#### Scenario: Ready node
Given a selected propagation node that is ready
When the Network screen renders
Then the readiness chip uses the positive tone
And the synchronization action sits beside the node select
And the airtime sentence remains available to describe the action

### Requirement: Boards scale with Dynamic Type

Board rows and chips must size from system text styles, and no chip may
overflow its card at the largest accessibility text size.

#### Scenario: Largest text size
Given the largest accessibility content size category
When the Network screen renders
Then each bearer row's name and chip scale together
And the chip wraps inside the card
