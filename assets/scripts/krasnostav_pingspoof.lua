-- krasnostav_pingspoof.lua
-- Highly optimized packet-level ping spoof utility for KRASNOSTAV Minecraft Client.
-- Delays outgoing network packets (like keepalives or transactions) safely to mimic high latency.

local script = {
    name = "PingSpoof",
    description = "Delays and buffers outgoing packets to simulate high network latency."
}

-- Trigger notification upon script loading
script:send_notification("success", "PingSpoof Loaded", "Lua PingSpoof script has been successfully initialized.")

-- Dynamically registers settings that the ClickGUI dashboard renders automatically
local delay_setting = script:registerSetting("Slider", "DelayMS", "The amount of ms to delay each packet for", 350.0)
local active_toggle = script:registerSetting("Toggle", "KeepAliveOnly", "Only delay keepalive packets", 1.0)

-- Subscribe to the client Event Bus for Game Tick and Chat events
script:subscribe_event_bus(1) -- Event::Tick
script:subscribe_event_bus(7) -- Event::Chat

-- Event hook for outgoing packets in JVM Netty channels
script:onPacketSend(function(packet)
    local name = packet:getName()

    -- Filter if KeepAliveOnly is checked
    if active_toggle:getValue() > 0.5 then
        if name == "CPacketKeepAlive" or name == "KeepAlive" then
            script:send_notification("progress", "Packet Delayed", "KeepAlive network packet suspended in native buffer.")
            return "DELAY"
        else
            return "FORWARD"
        end
    end

    return "DELAY"
end)

-- Update frame loop ticks
script:onUpdate(function(packets)
    local current_delay = delay_setting:getValue()

    for _, packet in ipairs(packets) do
        -- Check if packet has been aged longer than the configured threshold (e.g., 350ms)
        if packet:hasAged(current_delay) then
            -- Release packet from connection buffer onto JVM netty channel pipeline
            script:release(packet)
            script:send_notification("success", "Packet Released", "Successfully flushed buffered packet to JVM Netty pipeline.")
        end
    end
end)
