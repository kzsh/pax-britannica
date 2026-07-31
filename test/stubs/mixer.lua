-- Headless stand-in for the native `mixer` module.

local mixer = {}

local sound_mt = {__index = {
  play = function () return 0 end
}}

function mixer.init() return true end

function mixer.load_ogg() return setmetatable({}, sound_mt) end

function mixer.load_wav() return setmetatable({}, sound_mt) end

function mixer.channel_fade_to() end

return mixer
