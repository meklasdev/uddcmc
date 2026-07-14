-- krasnostav_pingspoof.lua
-- Highly optimized packet-level ping spoof utility for KRASNOSTAV Minecraft Client.
-- Delays outgoing packets (like keepalives or transactions) safely to mimic high latency.

local script = {
    name = "PingSpoof",
    description = "Delays all packets by a set amount of time."
}

-- Dynamically registers a setting that the ClickGUI dashboard must render
local delay = script:registerSetting("IntSetting", "Delay", "The amount of ms to delay each packet for", 500, 0, 5000)

-- Event hook for packet sending
script:onPacketSend(function(packet)
    -- Mutate or flag packet delay action
    return "DELAY"
end)

-- Update tick hook
script:onUpdate(function(packets)
    for _, packet in ipairs(packets) do
        -- Check if packet has been aged longer than the configured threshold (e.g. 1000ms)
        if packet:hasAged(1000) then
            -- Release packet from connection buffer onto JVM netty channel pipeline
            script:release(packet)
        end
    end
end)
