// Compile-time stub of io.netty.channel.ChannelHandlerContext — only the
// methods DarkChannelHandler calls. Signatures must match real Netty exactly,
// since the runtime links against the real class.
package io.netty.channel;

public interface ChannelHandlerContext {
    ChannelFuture write(Object msg, ChannelPromise promise);

    ChannelHandlerContext fireChannelRead(Object msg);
}
