::  |pill: helper functions for making pills
::
/-  *sept
^?
|%
::  a pill, the first events that a shrub sees, are:
::    - a card to metamorphose the dust-boot, which includes:
::      - our
::      - dust src
::      - dust formula
::      - vine src
::      - the special protocol %vase, which allows for duck-typing the
::      build system until the vine can bootstrap
::    - a card to bootstrap the vine. This is vine-specific, but usually
::    it means setting up the build system, and creating any default or
::    preconfigured binding necessary for the vine's operation
::
::    Note that unlike the urbit boot system, we use cards to multiplex
::    several pieces of information
+$  pill
  $:  %pill
      %brass
      boot-ova=(list)
      dust-boot=(list [%veer @t sove:vine:t])
      ~
      :: vine-boot=sove
  ==
+$  stem
  $:  %stem
      kernel=(list)
      dust-boot=(list [%veer @t sove:vine:t])
  ==
++  make-stem
  |=  [sys=path sur=path]
  ^-  stem
  =/  lib  (snoc (scag 3 sur) %lib)
  |^  =/  ver
        =/  sub  *(trap vase)
        =^  compiler-src  sub  (build-sys sub %hoon)
        =^  sept-src  sub  (build-sur sub | %sept)
        =^  kernel-src  sub  (build-lib sub | %neo)
        sub
        :: (build-lib sub | %neo)
      =/  nok  !.
        =>  *[ver=(trap vase) ~]
        !=  q:$:ver
      stem/[[nok ver ~] ~]
  ::
  ++  build-sys
    |=  [sub=(trap vase) nam=term]  ^-  [@t (trap vase)]
    ~>  %slog.[0 leaf+"ivory: building /sys/{(trip nam)}"]
    =/  src  .^(@t cx+(welp sys /[nam]/hoon))
    :-  src
    (swat sub (rain /sys/[nam]/hoon src))
  ++  build-sur
    |=  [sub=(trap vase) imp=? nam=term]  ^-  [@t (trap vase)]
    ~>  %slog.[0 leaf+"ivory: building /sur/{(trip nam)}"]
    =/  src  .^(@t cx+(welp sur /[nam]/hoon))
    =/  hun=hoon
      (mist /sur/[nam]/hoon src)
    :-  src
    ?.  imp  (swat sub hun)
    (swel sub [%ktts nam hun])
  ::
  ++  build-lib
    |=  [sub=(trap vase) imp=? nam=term]  ^-  [@t (trap vase)]
    ~>  %slog.[0 leaf+"ivory: building /lib/{(trip nam)}"]
    =/  src  .^(@t cx+(welp lib /[nam]/hoon))
    =/  hun=hoon
      (mist /lib/[nam]/hoon src)
    :-  src
    ?.  imp  (swat sub hun)
    (swel sub [%ktts nam hun])
  ::  +mist: +rain but skipping past ford runes
  ::
  ++  mist
    |=  [bon=path txt=@]
    ^-  hoon
    =+  vas=vast
    ~|  bon
    %+  scan  (trip txt)
    %-  full
    =;  fud
      (ifix [;~(plug gay fud) gay] tall:vas(wer bon))
    %-  star
    ;~  pose  vul
      %+  ifix  [fas (just `@`10)]
      (star ;~(less (just `@`10) next))
    ==
  ::  +swel: +swat but with +slop
  ::
  ++  swel
    |=  [tap=(trap vase) gen=hoon]
    ^-  (trap vase)
    =>  [tap=tap gen=gen ..zuse]
    ~>  %memo./pill/rain
    =/  gun  (~(mint ut p:$:tap) %noun gen)
    =>  [tap=tap gun=gun]
    |.  ~+
    =/  pro  q:$:tap
    [[%cell p.gun p:$:tap] [.*(pro q.gun) pro]]
  --
++  rain
  |=  [=path src=@]
  ~|  path
  =.  path  (snap path 2 '')
  =>  [path=path src=src ..zuse]
  ~>  %memo./pill/rain
  =+  vaz=vast
  (scan (trip src) (full (ifix [gay gay] tall:vaz(wer path))))
++  munt
  |=  =hoon
  =>  [hoon=hoon ..zuse]
  ~>  %memo./pill/munt
  q:(~(mint ut %noun) %noun hoon)
++  hoon-myth
  |=  hoon=@t
  (~(put by *myth:t) src:slot:t hoon/hoon)
::
++  dust-boot
  |=  $:  bas=path
          our=@
          hoon-src=@
          hoon-formula=(unit *)
          sept-src=@
          lain-src=@
          lain-formula=(unit *)
          vine-src=@
      ==
  ^-  [%veer src=@t sove:vine:t]
  :+  %veer  vine-src
  :+  #/sys/boot  /
  :-  %many
  %-  turn
  :_  |=  [=pith:t =myth:t]
      ^-  sard:vine:t
      [pith %make myth]
  ^-  (list (pair pith:t myth:t))
  =|  optional=(list (pair pith:t myth:t))
  =/  vine-tale  (~(put by *myth:t) src:slot:t hoon/vine-src)
  =/  hoon   (~(put by *myth:t) src:slot:t hoon/hoon-src)
  =/  lain-tale  (~(put by *myth:t) src:slot:t hoon/lain-src)
  =/  sept-tale  (~(put by *myth:t) src:slot:t hoon/sept-src)
  =?  hoon  ?=(^ hoon-formula)
    (~(put by hoon) vase:slot:t noun/u.hoon-formula)
  =?  lain-tale   ?=(^ lain-formula)
    (~(put by lain-tale) vase:slot:t noun/u.lain-formula)
     
  ^-  (list (pair pith:t myth:t))
  :~  [#/our (~(put by *myth:t) content:slot:t noun/our)]
      [#/sys/vine vine-tale]
      [#/sys/sept sept-tale]
      [#/sys/lain lain-tale]
      [#/sys/hoon hoon]
    ::
    :: compiled version goes in #/std/out/..
      :: [#/std/src/imp/ford-desk .^(@t %cx (weld bas /neo/imp/ford-desk/hoon))]
    ::  [#/std/src/imp/ford-face .^(@t %cx (weld bas /neo/imp/ford-face/hoon))]
    ::  [#/std/src/imp/ford-clay-src .^(@t %cx (weld bas /neo/imp/ford-clay-src/hoon))]
    ::  [#/std/src/imp/ford-reef .^(@t %cx (weld bas /neo/imp/ford-reef/hoon))]
    ::  [#/std/src/imp/ford-same .^(@t %cx (weld bas /neo/imp/ford-same/hoon))]
    ::  [#/std/src/imp/ford-slap .^(@t %cx (weld bas /neo/imp/ford-slap/hoon))]
    ::  [#/std/src/imp/ford-slop .^(@t %cx (weld bas /neo/imp/ford-slop/hoon))]
    ::  [#/std/src/imp/ford-text .^(@t %cx (weld bas /neo/imp/ford-text/hoon))]
    ::::
    ::  [#/std/src/pro/ford-desk .^(@t %cx (weld bas /neo/pro/ford-desk/hoon))]
    ::  [#/std/src/pro/ford-hoon .^(@t %cx (weld bas /neo/pro/ford-hoon/hoon))]
    ::  [#/std/src/pro/ford-poke .^(@t %cx (weld bas /neo/pro/ford-poke/hoon))]
    ::  [#/std/src/pro/ford-state .^(@t %cx (weld bas /neo/pro/ford-state/hoon))]
      [#/std/pro/hoon (hoon-myth .^(@t %cx (weld bas /neo/pro/hoon/hoon)))]
      [#/std/pro/pith (hoon-myth .^(@t %cx (weld bas /neo/pro/pith/hoon)))]
      [#/std/pro/atom (hoon-myth .^(@t %cx (weld bas /neo/pro/atom/hoon)))]
      [#/std/pro/tang (hoon-myth .^(@t %cx (weld bas /neo/pro/atom/hoon)))]
      [#/std/pro/stud (hoon-myth .^(@t %cx (weld bas /neo/pro/stud/hoon)))]
      [#/std/pro/sig (hoon-myth .^(@t %cx (weld bas /neo/pro/sig/hoon)))]
      [#/std/via/sell (hoon-myth .^(@t %cx (weld bas /neo/via/sell/hoon)))]
      [#/std/pro/noun (hoon-myth '!:  ,*')]
      [#/std/pro/vase (hoon-myth '!:  vase')]
      [#/std/pro/crew (hoon-myth '!:  (map term pith)')]
      [#/std/imp/noun (hoon-myth '~')]
      [#/std/imp/vase (hoon-myth '~')]
      [#/std/pro/piths (hoon-myth '!:  (list pith:t)')]
  ==
++  solid
  |=  bas=path
  =/  compiler-path  (weld (snoc bas %lib) /hoon)
  =/  sept-path      (weld (snoc bas %sur) /sept)
  =/  lain-path      (weld (snoc bas %lib) /shrub-arvo)
  =/  vine-path      (weld bas /lib/neo)
  ~&  %solid-start
  =/  compiler-src  .^(@t %cx (weld compiler-path /hoon))
  =/  sept-src  .^(@t %cx (weld sept-path /hoon))
  =/  lain-src
    %-  of-wain:format
    :-  ':: '     :: keep line numbers the same
    %+  slag  1   :: strip ford runes, TODO do more robustly
    (to-wain:format .^(@t %cx (weld lain-path /hoon)))
  =/  vine-src
    %-  of-wain:format
    :-  ':: '
    %+  slag  1
    (to-wain:format .^(@t %cx (weld vine-path /hoon)))
  ~&  %solid-compiling-compiler
  =/  compiler-hoon  (rain compiler-path compiler-src)
  =/  compiler-formula  (munt compiler-hoon)
  ~&  %solid-compiled-compiler
  =/  lain-formula
    =/  sept-hoon      (rain sept-path sept-src)
    =/  lain-hoon      [%tsgr sept-hoon (rain lain-path lain-src)]
    ::  compile arvo against hoon, with our current compiler
    ::
    =/  whole-hoon=hoon
      [%tsgr compiler-hoon [%tsgr [%$ 7] lain-hoon]]
    ~&  %solid-parsed
    =/  whole-formula  (munt whole-hoon)
    ~&  %solid-arvo
    whole-formula
  ::
  ::  kernel-formula
  ::
  ::    We evaluate :arvo-formula (for jet registration),
  ::    then ignore the result and produce .installed
  ::
  ::
  ::  boot-two: startup formula
  ::
  =/  boot-two
    =>  *[kernel-formula=^ tale=*]
    !=  [.*(0 kernel-formula) tale]
  ::
  ::  boot-ova
  ::
  =/  boot-ova=(list)
    [aeon:eden:part boot-two lain-formula ~]
  ::
  ::  a pill is a 3-tuple of event-lists: [boot kernel userspace]
  ::
  ::    Our kernel event-list is ~, as we've already installed them.
  ::    Our userspace event-list is a list containing a full %clay
  ::    filesystem sync event.
  ::
  =/  dus
    %:  dust-boot
        bas
        ~zod
        compiler-src
        `compiler-formula
        sept-src
        lain-src
        `lain-formula
        vine-src
    ==
  ::  install larval stage of vine so that the vine can finish installing itself
  :*  %pill  %brass
      boot-ova
      [dus ~]
      ~
  ==
--
