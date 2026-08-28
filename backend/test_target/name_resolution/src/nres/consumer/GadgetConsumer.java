package nres.consumer;

// Plain wildcard import: multi-level nested public types are reachable
// (relative to the package, so still qualified by their enclosing type),
// while Gadget.Hidden stays invisible.
import nres.wildcard.*;

public class GadgetConsumer {
    private Gadget gadget;
    private Gadget.Part part;
    private Gadget.Part.SubPart subPart;
}
