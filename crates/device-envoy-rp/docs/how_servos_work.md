# How Servos Work

When you call `set_degrees(45)`, the Pico starts sending a [PWM](crate#glossary) control signal to the
servo, telling it to move to and hold at 45 degrees. The Pico's hardware generates this control
signal automatically in the background, taking no CPU time. The control signal remains active until
you change it or call `relax()`.

The servo itself has no idea what angle it is currently at. It simply moves as fast as it can
to match whatever angle the current control signal specifies. The library does not—and cannot—wait
for the servo to reach a position. This is why you must wait (for example, using `Timer::after()`)
after calling `set_degrees()` to give the servo time to physically reach the target position.
