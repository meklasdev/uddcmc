/**
 * Netty pipeline handler injected into Minecraft's connection by DarkClient.
 *
 * Thin bridge: every outbound / inbound packet is handed to native (Rust) code
 * via {@link #onOutbound} / {@link #onInbound}, which returns the object to
 * forward — the same one, a replacement, or {@code null} to drop it. All
 * decision logic lives in Rust.
 *
 * Compiled against the stub Netty types in {@code stub/} (the real Netty
 * classes are resolved at runtime from Minecraft's class loader). The compiled
 * {@code DarkChannelHandler.class} is committed and embedded in the client
 * library. To rebuild after editing:
 *
 *   javac --release 17 -d <out> stub/io/netty/channel/*.java DarkChannelHandler.java
 *   cp <out>/DarkChannelHandler.class .
 */
public class DarkChannelHandler extends io.netty.channel.ChannelDuplexHandler {

    private static native Object onOutbound(Object packet);

    private static native Object onInbound(Object packet);

    @Override
    public void write(io.netty.channel.ChannelHandlerContext ctx, Object msg,
                      io.netty.channel.ChannelPromise promise) throws Exception {
        Object result;
        try {
            result = onOutbound(msg);
        } catch (Throwable ignored) {
            result = msg;
        }
        if (result != null) {
            ctx.write(result, promise);
        }
    }

    @Override
    public void channelRead(io.netty.channel.ChannelHandlerContext ctx, Object msg)
            throws Exception {
        Object result;
        try {
            result = onInbound(msg);
        } catch (Throwable ignored) {
            result = msg;
        }
        if (result != null) {
            ctx.fireChannelRead(result);
        }
    }
}
