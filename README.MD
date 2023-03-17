# Jim's Overlord
Jim's Overlord is a control program for the remote controlled lawn mower called Jim. It opens a websocket on port `7032` and uses controllers that are connected to the overlord pc to send a control signal over the websocket.

The control signal is in the format `L00R00M00` Where the `0` can be `1-9` and indicate the speed of the relative motors:
    - L: Left motor
    - R: Right motor
    - M: Mower motor

Currently the controller mapping is left trigger for left motor, right trigger for right motor and `x` on an xbox controller to engage the mower motor.