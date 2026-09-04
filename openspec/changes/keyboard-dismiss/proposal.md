# Keyboard Dismiss

## Intent

Let the operator leave a draft without sending it. On iOS a focused text
field keeps the keyboard up until something blurs it, and the shell hides
the tab bar while the keyboard is up, so every path out of a draft was
blocked.

## Scope

The mobile page template in `apps/mobile` and the composer in
`styrene-ui-app`: a page-level tap-outside listener that blurs a focused
form control, and a Done control beside Send that is visible only while the
keyboard is up.

## Success criteria

- Tapping anywhere that is not a form control or its label dismisses the keyboard.
- A Done control is visible beside Send while the keyboard is up and dismisses it.
- The tab bar returns as soon as the keyboard is dismissed.
