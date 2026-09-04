package nres.diamond;

// Same diamond shape, but this type declares its own "Shared" nested type,
// which must shadow (not conflict with) IfaceA's inherited one.
public class ShadowsShared implements IfaceA {
    public static class Shared {
        public String tag = "own";
    }
}
