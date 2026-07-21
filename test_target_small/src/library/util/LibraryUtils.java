package library.util;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

import library.core.Identifiable;

public final class LibraryUtils {
    private LibraryUtils() {
    }

    public static void printAll(List<? extends Identifiable<String>> items) {
        for (Identifiable<String> item : items) {
            System.out.println(" - " + item.getId());
        }
    }

    public static <T extends Comparable<T> & Identifiable<String>> List<T> sorted(List<T> items) {
        List<T> copy = new ArrayList<>(items);
        copy.sort(Comparator.naturalOrder());
        return copy;
    }
}
