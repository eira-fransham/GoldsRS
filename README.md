# GoldsRS

An uncoordinated mess of HL1 format parsers. Currently this actually only parses
(well, "parses" is too strong a term for lazily blitting directly from a memmap)
Quake 1 BSPs, but I'm hoping to make the parser generic so that I can use the
same interface for Quake 1, Quake 2 and HL1/Goldsrc. They're very similar to
eachother so I expect to be able to make a consistent interface for all 3.
Ideally I'd also like to be able to load Quake 3 BSPs too, but I won't sweat it
if that turns out to be a bunch of work. Honestly I just want to get HL1 levels
working and then I'll move on to the other formats.

It's worth noting that this code is unsafe as holy hell. You can't open security
holes with it (you can unsafely read into unintended areas but not write), but
it's really easy to get utterly meaningless data.
