package nres.consumer;

// Static wildcard import: Palette's own public static nested types are
// bound relative to Palette itself (bare "Color", not "Palette.Color"),
// and Palette.Secret stays invisible.
import static nres.staticimport.Palette.*;

public class ThemeConsumer {
    private Color color;
}
