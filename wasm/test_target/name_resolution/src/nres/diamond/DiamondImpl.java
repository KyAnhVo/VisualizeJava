package nres.diamond;

// Implements two unrelated interfaces that each export a nested type named
// "Shared" -- this makes DiamondImpl's own exported-nested-type map mark
// "Shared" as Ambiguous. Deliberately nothing (here or elsewhere in this
// fixture) references that bare ambiguous name, since doing so is the one
// path in the resolver that still hard-panics.
public class DiamondImpl implements IfaceA, IfaceB {
}
