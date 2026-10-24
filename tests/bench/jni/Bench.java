// Issue #15 prototype (not part of the app): see run.sh.
import java.text.SimpleDateFormat;
import java.util.*;
import org.json.*;

public class Bench {
    static native String franchiseRust(String title);
    static native double[] weatherRust(String body, double now);

    // Recs.franchise, as in the app.
    static String franchise(String title) {
        String t = title == null ? "" : title.toLowerCase(Locale.ROOT);
        int cut = t.indexOf(':');
        if (cut < 0) cut = t.indexOf(" - ");
        return (cut > 0 ? t.substring(0, cut) : t).trim();
    }

    // HomeData.parseWeather, as in the app, packed the same way as the Rust's answer.
    static double[] weather(String body, double now) throws Exception {
        JSONObject o = new JSONObject(body);
        TimeZone zone = TimeZone.getTimeZone(o.optString("timezone", TimeZone.getDefault().getID()));
        List<Double> out = new ArrayList<>();
        JSONObject current = o.getJSONObject("current");
        out.add(current.getDouble("temperature_2m")); out.add((double) current.getInt("weather_code"));
        out.add(current.optInt("is_day", 1) == 1 ? 1.0 : 0.0);
        JSONObject daily = o.getJSONObject("daily");
        out.add(daily.getJSONArray("temperature_2m_max").getDouble(0)); out.add(daily.getJSONArray("temperature_2m_min").getDouble(0));
        out.add((double) local(daily.getJSONArray("sunrise").getString(0), zone)); out.add((double) local(daily.getJSONArray("sunset").getString(0), zone));
        JSONObject hourly = o.optJSONObject("hourly");
        JSONArray times = hourly.getJSONArray("time"), temps = hourly.getJSONArray("temperature_2m"), codes = hourly.getJSONArray("weather_code");
        JSONArray days = hourly.optJSONArray("is_day");
        int n = 0;
        for (int i = 0; i < times.length() && n < 6; i++) {
            long at = local(times.getString(i), zone);
            if (at <= now) continue;
            n++;
            out.add((double) at); out.add(temps.getDouble(i)); out.add((double) codes.getInt(i)); out.add(days == null || days.optInt(i, 1) == 1 ? 1.0 : 0.0);
        }
        double[] r = new double[out.size()];
        for (int i = 0; i < r.length; i++) r[i] = out.get(i);
        return r;
    }

    static final SimpleDateFormat F = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm", Locale.US);
    // The same, with one formatter kept (Java-only fix: no new SimpleDateFormat per hour).
    static long local(String iso, TimeZone zone) throws Exception {
        if (!F.getTimeZone().getID().equals(zone.getID())) F.setTimeZone(zone);
        return F.parse(iso).getTime();
    }

    interface Call { Object run(int i) throws Exception; }

    static double ns(Call c, int runs) throws Exception {
        for (int i = 0; i < 20000; i++) c.run(i);   // warm up the JIT
        long t = System.nanoTime();
        for (int i = 0; i < runs; i++) c.run(i);
        return (System.nanoTime() - t) / (double) runs;
    }

    public static void main(String[] a) throws Exception {
        System.loadLibrary("fmtv_jni");
        long now0 = System.currentTimeMillis();
        StringBuilder t = new StringBuilder(), tc = new StringBuilder(), cd = new StringBuilder(), dy = new StringBuilder();
        SimpleDateFormat f = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm", Locale.US);
        f.setTimeZone(TimeZone.getTimeZone("UTC"));
        for (int h = 0; h < 48; h++) {
            String sep = h == 0 ? "" : ",";
            t.append(sep).append('"').append(f.format(new Date(now0 + (h - 12) * 3600000L))).append('"');
            tc.append(sep).append(10 + h / 3.0); cd.append(sep).append(h % 4); dy.append(sep).append(h % 2);
        }
        String body = "{\"timezone\":\"UTC\",\"utc_offset_seconds\":0,\"current\":{\"temperature_2m\":21.4,\"weather_code\":3,\"is_day\":1},"
            + "\"daily\":{\"temperature_2m_max\":[25.1],\"temperature_2m_min\":[14.2],\"sunrise\":[\"2026-10-20T05:30\"],\"sunset\":[\"2026-10-20T18:01\"]},"
            + "\"hourly\":{\"time\":[" + t + "],\"temperature_2m\":[" + tc + "],\"weather_code\":[" + cd + "],\"is_day\":[" + dy + "]}}";
        double now = now0;
        // Same answers first.
        String[] titles = {"Law & Order: Special Victims Unit", "Dune - Part Two", " ÉLITE ", ":x", ""};
        for (String s : titles) if (!franchise(s).equals(franchiseRust(s))) throw new AssertionError("franchise " + s);
        if (!Arrays.equals(weather(body, now), weatherRust(body, now))) throw new AssertionError("weather differs");
        System.out.println("Same answers: yes");
        double fj = ns(i -> franchise(titles[i % 5]), 2_000_000), fr = ns(i -> franchiseRust(titles[i % 5]), 2_000_000);
        double wj = ns(i -> weather(body, now), 20_000), wr = ns(i -> weatherRust(body, now), 20_000);
        System.out.printf("| Recs.franchise | %.0f ns | %.0f ns | %.1fx |%n", fj, fr, fr / fj);
        System.out.printf("| HomeData.parseWeather (48 h) | %.1f us | %.1f us | %.1fx |%n", wj / 1000, wr / 1000, wr / wj);
    }
}
