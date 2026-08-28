package nres.consumer;

// Precedence chain: wildcard import < single-type import < same-file
// declaration. Per the README, "Token" declared below must win.
import nres.shadow1.*;
import nres.shadow2.Token;

public class Token {
}
