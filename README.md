# Shrine on nock research prototype

> I met a traveller from an antique land,
> Who said—“Two vast and trunkless legs of stone
> Stand in the desert. . . . Near them, on the sand,
> Half sunk a shattered visage lies, whose frown,
> And wrinkled lip, and sneer of cold command,
> Tell that its sculptor well those passions read
> Which yet survive, stamped on these lifeless things,
> The hand that mocked them, and the heart that fed;
> And on the pedestal, these words appear:
> My name is Ozymandias, King of Kings;
> Look on my Works, ye Mighty, and despair!"
> Nothing beside remains. Round the decay
> Of that colossal Wreck, boundless and bare
> The lone and level sands stretch far away.

## Overview

This repo is an attempt to make the Shrine platform work using nock as its
substrate. It is being released as a research object as nock is no longer
considered viable for this use case.

## License

Copyright is retained by Axiomatic Systems. Licensed under the AGPLv3.

### Problems

#### Memory management

Due to the amount of garbage created, and nock's dependence on hash-consing, a
performant nock runtime requires control of its allocations. The control
required is quite deep, as the allocator in vere is complex. This makes any
jetting of operations that would like to manipulate the representation of
objects in the VM a cross cutting concern.

#### Path interning

Keeping a versioned filesystem that tracks three separate version
numbers for each node is complicated. It requires operations on paths to be as
rapid as possible, as well as the paths themselves being interned, so that they
can be packed tightly in memory. In this repo, this necessitates swapping to
and from the interned representation and the noun representation every time you
load or save from disk. Additionally, because vere's allocator is single
threaded, this must be done on the main thread (and it blocks the DB, because
inserting is a write op). Moreover, because the interned path ids block the DB,
we also use the interning as an opportunity to cache the parent, which will not
change over the lifecycle of the path. Unfortunately, because slots are not
required to exist or be synchronised, the slot representation falls back to a
string.

The obvious solution is something like PLAN's pins. Extending nock to support
something pin-like runs into all of the issues outlined above with data jets
and is likely not feasible. Moreover, you're already hash-consing everything in
the name of performance, so the necessity of another layer of hash-addressing is
deeply suspect from an implementation POV.

#### Stateful jet registration

Because jet registration is stateful, a shrine implementation must two one of
two things:
- Have all build systems output a `(trap vase)` instead of just a vase. This is
  deeply awful because if you do not globally deduplicate (which blocks the the
  thread), then you will end up with jet state that is 10-100x than
- Accept that nothing outside the stdlib will be jetted, and deduplicate vases.

We chose the latter, writing a variant of jam/cue (jim/mew) which deduplicates
the inclusion of the stdlib when cueing. This works, but is not great.

#### Monokernel vs microkernel

Nock is fundamentally designed to be a single threaded event processor. This has
the unfortunate consequence of presupposing the question of "what is an event".
The answer to this question is context dependent, and Urbit sidesteps this by
ensuring that the context is always a "server shaped personal computer that's attached to
the internet". This fundamentally limits its scale and ambition. What counts as
an event that should cause a state transition is fundamentally different
depending on the scale of the computer you're thinking about.

"That's fine, we can just have different distribution's depending on usecase"

The issue here is that a nock kernel has two sides. It has the nock defined
event processing, and it has the (typically C) event construction.

> sidebar:
> yes, it's possible to define the event construction in nock, but nock has no
> impure escape hatch that would make untangling libuv's event spaghetti
> tractable. It also just isn't fast enough to write the whole system this way.

Writing the event construction in C splits any given functionality into two
faces. These faces must be kept in sync, and unlike jets there is nothing good
for keeping them in sync. This presents an issue when:
- (a) the computer would like to change scale
- (b) the computer would like to interact with another computer of a different
  scale

Let's consider the first, and what specifically is meant by "changing scale".
This actually occurs in arvo right now in ames, which is one of the worst
victims of the bifaciality problem as both the C driver and the kernel vane deal
with multiple different timescales (packet-like, and message-like).

Consider a flow transceiving heavy traffic. Obviously the runtime would like to
detect this and inform the vane that this flow will have its packets batched, or
even that it will only inform the vane of complete requests/responses. Under a
bifacial model, the packet pump must be implemented twice, once for each side.

This bifaciality program is much of why interoperation from mars to earth is so
complex.


### Getting started
This uses git submodules, please run

#### hoon
The hoon is all in the shrub/ subdirectory. Refer to build-help.hoon which is
used to produce the boot pill.

#### Rust
/sys and /vere contain the rust ffi layer over the vere runtime.

There is a rust testing layer in shrine-test, and some debugging infra in
shrine-debug.

shrine-core and shrine-storage define most of the system, althogh the layering
between the two is deeply suspect (mostly an artifact of path-interning).


### Bugs and issues

All of these should be considered wontfix.
- Reactions and the supervisor tree are buggy
- run with RUST_LOG="trace" to get slog output. unfortunately, the slog output
  is the raw pretty printed tang, (i.e. as linked list of ascii), which means
  you have to copy and paste it into a ship to make it work.




