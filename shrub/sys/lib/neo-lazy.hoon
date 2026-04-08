!:
=>
~%  %neon  +  ~
|%
::  XX: deprecated, remove
+$  disk  $@(~ [=ship =desk])
::
++  pro
  |%
  +$  atom  %atom
  +$  vase  %vase
  +$  noun  %noun
  +$  txt   %txt
  +$  duct  %duct
  +$  move  %/std/pro/move
  +$  card  %/std/pro/card
  +$  hoon  %hoon
  --
++  peek-rom
  |=  hun=hunt:t
  ^-  epic:t
  %-  need
  (peek-case 0 [care pith]:hun)
++  peek-parent
  |=  pax=pith:t
  ^-  (unit pith:t)
  =/  =epic:t  (peek-rom %x (welp /parents pax))
  ?~  fil.epic  ~
  =/  pal=(unit pail:t)  (~(get by q.u.fil.epic) content:slot:t)
  ?~  pal  ~
  ?.  ?=([%pith *] u.pal)  ~
  `+.u.pal
++  salt
  |=  t=tang
  ^+  !!
  =/  w=(list cord)  (tang-to-wain t)
  |-  ^+  !!
  ?~  w  !!
  =/  i=cord  i.w
  ~>  %mean.i
  $(w t.w)

++  slug
  |=  t=tang
  ^+  same
  =/  w=(list cord)  (tang-to-wain t)
  |-  ^+  same
  ?~  w  same
  ~>  %slog.[0 i.w]
  $(w t.w)


++  tang-to-wain
  |=  =tang
  ^-  (list cord)
  %-  zing
  %+  turn  tang
  |=  t=tank
  (turn (wash [0 80] t) crip)




++  strip
  |=  =epic:t
  ^-  muck:t
  (~(run of epic) tail)
++  rent
  |=  a=*
  ^-  tank
  :-  %leaf
  |-  ^-  tape
  ?@  a  (scow %ud a)
  %-  zing
  :~  "["
      $(a -.a)
      " "
      $(a +.a)
      "]"
  ==
::
++  peek-case
  |=  [case=@ud hun=hunt:t]
  ^-  (unit epic:t)
  =+  .^(res=(unit epic:t) case [care pith]:hun)
  ?^  res
    ?>  ~(apt of u.res)
    res
  res
      ::~>  %slog.[0 (crip ())]
  ::~>  %mean.|-((rent foo))
  ::;;((unit epic:t) foo)

++  neon-prelude  %prelude
++  spat  en-cord:pith:t
+$  octs  (pair @ud @)                                  ::  octet-stream
++  scars  (~(gas in *(set stud:t)) ~[%hoon %txt %cant %crew %duct %piths %pith %rpith %atom %path])
::
++  sug
  |_  =tale:t
  ++  vase  `(unit pail:t)`(~(get by tale) vase:slot:t)
  ++  res
    (~(get by tale) res:slot:t)
  ++  kook
    ^-  (list pith:t)
    =/  kok  (~(get by tale) kook:slot:t)
    ?~  kok  *(list pith:t)
    ?.  ?=(%duct -.u.kok)  ~
    +.u.kok
  ++  hoon
    ^-  (unit @t)
    ?~  pal=(~(get by tale) src:slot:t)  ~
    ?.  ?=(%hoon-src -.u.pal)  ~
    `+.u.pal
  ++  content
    (~(get by tale) content:slot:t)
  ++  crew
    ^-  crew:t
    =/  cru  (~(get by tale) crew:slot:t)
    ~&  cru/cru
    ?~  cru  *crew:t
    ?>  ?=(%crew -.u.cru)
    +.u.cru
  ++  fans
    =/  fan  (~(get by tale) fans:slot:t)
    ?~  fan  *fans:t
    ?>  ?=(%fans -.u.pro)
    +.u.fan
  --

++  sup
  |_  =tale:t
  ++  clue
    |=  m=clue:n
    (~(put by tale) card:slot:t %noun m)
  ++  content
    |=  =pail:t
    (~(put by tale) content:slot:t pail)
  ++  vase
    |=  v=^vase
    (~(put by tale) vase:slot:t %vase v)
  ++  build-fail
    |=  ta=tang
    (~(put by tale) vase:slot:t %tang ta)
  ++  move
    |=  s=move:vine
    (~(put by tale) card:slot:t %noun s)
  ++  hoon
    |=  txt=@t
    (~(put by tale) src:slot:t %hoon-src txt)
  --
++  scarify
  |=  =pail:t
  ^-  pail:t
  ?:  (~(has in scars) -.pail)
    pail
  noun/'aaa'
  ::=>  [pail=pail ..ride]
  ::~>  %memo./lain/rent/next
  ::?:  (gth (met 3 (jam q.q.pail)) 8.192)
  ::  /std/pro/tang^~[leaf/"too-big"]
  :::-  /std/pro/tang
  ::^-  tang
  ::?+  p.pail  [(sell q.pail) ~]
  ::  %tang  !<(tang q.pail)
  ::  %tank  [!<(tank q.pail) ~]
  ::==

++  is-pail-eq
  |=  [a=pail:t b=pail:t]
  =(a b)
::
++  is-tale-eq
  |=  [a=tale:t b=tale:t]
  =(a b)
::
++  is-tale-super
  |=  [sup=tale:t sub=tale:t]
  ::  ~>  %bout.[1 %tale-super]
  %+  levy  ~(tap by sub)
  |=  [=slot:t =pail:t]
  ^-  ?
  ?~  n=(~(get by sup) slot)  |
  (is-pail-eq u.n pail)
++  slub  (slib prelude)

++  slib
  |=  pre=vase
  |=  [=pith:t txt=@t]
  ^-  (each pail:t tang)
  =+  vaz=(vang & (pout pith))
  %-  mule  |.
  :-  *vase:pro
  %+  slap  pre
  %+  scan  (trip txt)
  (full (ifix [gay gay] tall:vaz))
::
++  slub-imp  (slib-imp prelude)
++  slib-imp
  |=  pre=vase
  |=  [=pith:t txt=@t]
  ^-  (each pail:t tang)
  =/  nil  ((slib pre) pith txt)
  ::?:  &(?=(%& -.nil) =(~ q.p.nil))
    ::[%& *vase:pro^!>(~)]
  %-  mule  |.
  =+  vaz=(vang & (pout pith))
  =|  =spec:n
  =/  =hoon
    (scan (trip txt) (full (ifix [gay gay] tall:vaz)))
  ::=/  [=spec:n =hoon]
    ::(rash txt (entry:preprocess pith))
  :-  %vase
  %+  slop  !>(spec)
  (slap pre hoon)

++  n
  ^?
  |%
  +$  gift  (axal mode:t)
  +$  crew  (map term pith:t)
  +$  fans  (jug care:t pith:t)
  +$  move  move:v
  +$  card  card:v
  +$  omen
    $~  [%dead ~ ~]
    $^  [p=omen q=omen]
    $%  lard:v
        amen
    ==
  +$  amen
    $%  [%dead p=pith:t =slot:t]
        [%wack p=pith:t =flow err=(unit tang)]
        [%take p=pith:t =gift]
        [%hear p=pith:t =rely]
    ==
  +$  true
    $%  lard:v
        amen
    ==
  +$  clue  (pair pith:t omen)
  ::  linearized clue
  +$  flue  (pair pith:t true)
  +$  flow  [src=pith:t dst=pith:t]
  +$  line
    $%  [%dead =slot:t]
        [%wack =flow err=(unit tang)]
        [%take =gift]
        [%hear =rely]
    ==
  +$  sign
    $%  note:v
        line
    ==
  +$  curb
    $~  [%any ~]
    $%  [%any ~]
        [%or (set stud:t)]
    ==
  +$  flaw  (pair term vice)

  ::  $vice: Polymorphism failure trace
  +$  vice
    $%  [%gone ~] :: injected dependency is nonexistent
        [%lost ~] :: couldn't find required dependency
        [%vair ~] :: no path to polymorphise
        [%pook p=(set stud:t)] :: missing pokes
    ==
  ::  XX: shit, change
  +$  rely  (set pith:t)

  +$  soma  (xcel lash)
  ++  xcel
    |$  [item]
    [fil=(unit item) kid=(map zeta $)]
  +$  zeta  (each iota aura)
  +$  norma  (xcel tale:t)
  +$  rail  $+  rail  (list zeta)
  ++  band  $+(band (map term fief))
  +$  lash  [state=curb poke=(set stud:t)]
  +$  fief  [=deed =care:t =soma]
  +$  wave  [live=? =crew fans=(jug care:t pith:t)]
  +$  deed
    $:  time=(unit @dr)
        req=?
    ==
  +$  link
    $%  [%path =pith:t]
        [%spec =wing]
        [%hoon =wing]
    ==
  +$  knit  (pair care:t yoke)
  +$  yoke  (each epic:t pith:t)
  +$  echo
    $:  imm=(map slot:t (set stud:t))
        dep=(map slot:t care:t)
        $=  run
        $-([imm=(map slot:t pail:t) dep=(map slot:t pith:t)] muck:t)
    ==
  +$  lect  $@(cord link)
  +$  help  (list lect)
  ::  $cant: secret language of the renderer
  ::
  ::    specifies a default way to render and item and it's children
  +$  cant  ?(%doc %chat %table %filetree)
  ++  get-true-pith
    |=  tr=true
    ^-  pith:t
    p.tr
  ++  card-is-down
    |=  [from=pith:t c=card:v]
    ^-  ?
    ?@  -.c
      (is-parent:u from p.c)
    ?&($(c -.c) $(c +.c))
  ++  card-to-omen
    |=  c=card:v
    ^-  omen
    ?@  -.c
      c
    [$(c -.c) $(c +.c)]
  ++  card-to-lards
    |=  c=card
    ^-  (list lard:v)
    ?@  -.c
      ~[c]
    (welp $(c -.c) $(c +.c))
  ++  true-to-sign
   |=  o=true
   ^-  sign
    ?-  -.o
      %dead  [%dead slot.o]
      %wack  [%wack flow.o err.o]
      %take  [%take gift.o]
      %hear  [%hear rely.o]
      %poke  [%poke tale.o]
      %make  [%make tale.o]
      %cull  [%cull ~]
    ==

  ++  omen-to-true
    |=  o=omen
    ^-  (list true)
    ?@  -.o
      %-  limo
      :_  ~
      ^-  true
      ?-  -.o
        %dead  [%dead p.o slot.o]
        %wack  [%wack p.o flow.o err.o]
        %take  [%take p.o gift.o]
        %hear  [%hear p.o rely.o]
        %poke  [%poke p.o tale.o]
        %make  [%make p.o tale.o]
        %cull  [%cull p.o ~]
      ==
    (welp $(o -.o) $(o +.o))
  ::  +clue-to-flues: linearize clue
  ++  clue-to-flues
    |=  c=clue
    ^-  (list flue)
    =/  o  q.c
    |-  ^-  (list flue)
    ?@  -.o
      %-  limo
      :_  ~    ^-  flue
      :-  p.c  ^-  true
      ?-  -.o
        %dead  [%dead p.o slot.o]
        %wack  [%wack p.o flow.o err.o]
        %take  [%take p.o gift.o]
        %hear  [%hear p.o rely.o]
        %poke  [%poke p.o tale.o]
        %make  [%make p.o tale.o]
        %cull  [%cull p.o ~]
      ==
    (welp $(o -.o) $(o +.o))
  ++  ford
    |%
    ::++  is-stud
    ::  |=  s=stud
    ::  ?^  s  |
    ::  =(%ford (end [3 4] s))
    ::  +riff:ford: Constant build system node
    ::
    ::    Required for bootstrapping. This is used to put the reef and
    ::    other ford combinators into the build system to bootstrap
    ::    everything else. To update a riff, simply %make over the top
    ::
    ::++  riff
    ::  |%
    ::  ++  state  %ford-out
    ::  ++  poke   *(set stud)
    ::  ++  kids  ~
    ::  ++  deps  ~
    ::  ++  form
    ::    ^-  ^form
    ::    |_  [=bowl =ever state-vase=vase *]
    ::    +*  sta  !<([cache=(unit vase) ~] state-vase)
    ::    ++  poke
    ::      |=  =pail
    ::      ^-  (quip card vase)
    ::      !!
    ::    ::
    ::    ++  init
    ::      |=  old=(unit vase)
    ::      ^-  (quip card vase)
    ::      =+  !<(ref=vase (need old))
    ::      `!>(`[cache=(unit vase) ~]`[`ref ~])
    ::    --
    ::  --
    ::  +dep:ford: $fief for a ford dependency
    ::
    ::    Handy shortcut to specifiy a dependency in the build system
    ++  dep  `fief`[dep-deed %x [`dep-lash ~]]
    ++  dep-lash  `lash`[or/(sy %ford-out %ford-in ~) ~]
    ++  dep-deed  `deed`[~ &]
    ::  +get-output: pull build resuit of dependency
    ::
      :: =+  !<([vax=(unit vase) *] outer)
    ::
    ++  run
      |=  txt=@t
      (scan (trip txt) (rein *name))
    +$  loc
      [=disk =pith]
    ::  $lib:ford: Specification of library import
    ::
    +$  lib
      [face=term =loc]
    ::  $pro:ford: Specification of protocol import
    ::
    +$  pro
      [face=term =stud]
    +$  vale
      [face=term =stud]
    ::  $file:ford: Code with imports
    ::
    +$  file
      $:  pro=(list pro)
          :: grab=(list
          lib=(list lib)
          =hoon
      ==
    ::  +rein:ford: Parse code with imports
    ++  rein
      |=  =name
      =<  apex
      |%
      ++  dis
        ;~  pose
          (cold ~ cab)
          ;~((glue bar) ;~(pfix sig fed:ag) sym)
        ==
      ::  +lib-loc: Parse library location
      ::
      ++  lib-loc
        ;~(plug dis stip)
      ::  +old-nam: Parse file path (deprecated: XX revisit)
      ::
      ::     Examples:
      ::     Absolute ~hastuc-dibtux/src/foo/bar/test
      ::     Relative %^/bar
      ++  old-nam
        :: ^-  $-(nail (like name:neo))
        ;~  pose
          %+  sear
            |=  [kets=(list) pit=pith]
            ^-  (unit ^name)
            %-  mole
            |.  ^-  ^name
            =.  pit  (scag (sub (lent pit) (lent kets)) pit)
            =-  ~&(parsed-name/- -)
            name(pith (welp pith.name pit))
          ;~(pfix cen ;~(plug (star ket) stip))                     :: relative
          ;~(plug ;~(pfix fas sig fed:ag) stip) :: absolute
        ==
      ::  +std:rein:ford: Parse import directive
      ::
      ::    Either  name:~ship/desk
      ::    or      name (from %std disk)
      ::
      ++  std
        ;~  pose
          :: ;~(plug sym ;~(pfix col sig fed:ag) ;~(pfix fas sym))
          sym
        ==
      ::  +pro:rein:ford: Parse protocol import directive
      ::
      ::    /@  foo=bar  :: imports %bar protocol from %std disk with name foo
      ::    /@  bar      :: imports %bar protocol from %std disk with name bar
      ::
      ++  pro
        :: ^-  $-(nail (like ^pro))
        %+  rune  pat
        ;~  pose
          ;~(plug sym ;~(pfix tis std))
          %+  cook
            |=  =stud
            ?@  stud  [stud stud]
            [i.stud stud]
          std
        ==
      ++  lib
        %+  rune  hep
        ;~  pose
          ;~(plug sym ;~(pfix tis lib-loc))
          %+  cook
            |=  [=disk =pith]
            ^-  ^lib
            =/  last  (rear pith)
            ?>  ?=(@ last)
            [`@tas`last disk pith]
          lib-loc
        ==
      ::  +old-lib: Parse arbitrary library import directive
      ::
      ::    Unused, todo revive with more recursive build system
      ::
      ::    /-  face=~hastuc-dibtux/foo/bar <- imports ~hastuc-dibtux
      ::    /-  %^/bar <- imports bar/hoon up one level with face bar
      ::
      ++  old-lib
        :: ^-  $-(nail (like ^lib))
        ::
        %+  rune  hep
        ;~  pose
          ;~(plug sym ;~(pfix tis old-nam))
          %+  cook
            |=  n=^name
            =/  last  (rear pith.n)
            :_  n
            ?@  last  last
            (scot last)
          old-nam
        ==
      ++  rune
        |*  [car=rule rul=rule]
        (ifix [;~(plug fas car gap) gay] rul)

      ++  libs
        :: ^-  $-(nail (like (list ^lib)))
        (star lib)
      ++  pros
        :: ^-  $-(nail (like (list ^pro)))
        (star pro)
      ++  hone
        :: ^-  $-(nail (like hoon))
        =+  vaz=(vang & (en-path:^name name))
        (ifix [gay gay] tall:vaz)
      ++  apex
        :: ^-  rule
        ;~  plug
          pros
          libs
          hone
        ==
      --
    ::  +with-face:ford: Decorate vase with face
    ::
    ++  with-face
      |=  [fac=@tas =vase]
      vase(p [%face fac p.vase])
    ::  +with-faces:ford: Decorate vases with faces, slopped onto reef
    ::
    ++  with-faces
      |=  [reef=vase faces=(list (pair term vase))]
      ?~  faces
        reef
      $(reef (slop (with-face i.faces) reef), faces t.faces)
    --
  ++  bowl
    $:  src=pith:t
        here=pith:t
        now=@da
        eny=@uv
        deps=(map term (pair pith epic:t))
        kids=muck:t
    ==
  ++  strap-build
    ^-  form
    =>
      |%
      ++  get-sut
        |=  =bowl
        ~>  %slog.[0 (crip ~(ram re >~(key by deps.bowl)<))]
        ?~  dep=(~(get by deps.bowl) %sut)
          ~>  %slog.[0 %no-sut-in-dep]
          prelude
        =/  =tale:t  q:(need fil.q.u.dep)
        ~>  %slog.[0 (crip ~(ram re >~(key by tale)<))]
        ?~  vax=(~(get by tale) vase:slot:t)
          ~>  %slog.[0 %no-vax-in-sut]
          prelude
        ?.  ?=(%vase -.u.vax)
          ~>  %slog.[0 %bad-sut-vax]
          prelude
        +.u.vax
      ::
      ++  compile
        |=  [=pith:t sut=vase txt=@t]
        ^-  pail:t
        =;  res=(each vase tang)
          ?:  ?=(%& -.res)
            [%vase p.res]
          %-  (slug p.res)
          [%wain (tang-to-wain p.res)]
        %-  mule   |.
        =/  =hoon  (rain (pout pith) txt)
        (slap sut hoon)
      ++  build
        |=  [=bowl =tale:t]
        ^+  tale
        ?~  hon=(~(get by tale) src:slot:t)
          tale
        =/  sut=vase  (get-sut bowl)
        ?>  ?=(%cord -.u.hon)
        %+  ~(put by tale)  vase:slot:t
        (compile here.bowl sut +.u.hon)
      --
    |_  [=bowl =saga:t]
    ++  init
      |=  =tale:t
      ^-  (quip card tale:t)
      `(build bowl tale)
    ++  talk
      |=  =tale:t
      ^-  (quip card tale:t)
      `(build bowl (~(uni by q.saga) tale))
    ++  take   take:def
    ++  dead   dead:def
    ++  hear
      |=  =rely
      ^-  (quip card tale:t)
      `(build bowl q.saga)
    ++  goof   goof:def
    --
  ++  def
    ^-  form
    |_  [=bowl =saga:t]
    ::  handle initial
    ++  init
      |=  =tale:t
      ^-  (quip card tale:t)
      `tale
    ::  handle change to state
    ++  talk
      |=  =tale:t
      ^-  (quip card tale:t)
      `(~(uni by q.saga) tale)
    ::  handle change to children
    ++  take
      |=  =gift
      ^-  (quip card tale:t)
      `q.saga
    ::  handle death of dependency
    ++  dead
      |=  =slot:t
      ^-  (quip card tale:t)
      `q.saga
    ::  hear: handle depnendency change
    ++  hear
      |=  =rely
      ^-  (quip card tale:t)
      `q.saga
    ::  goof: handle possible error
    ++  goof
      |=  [c=card err=(unit (list tank))]
      ^-  (quip card tale:t)
      `q.saga
    --
  ::
  ++  form
    $+  form
    $_  ^|
    |_  [=bowl =saga:t]
    ::  +init: start shrub
    ++  init
      |~  =tale:t
      *(quip card tale:t)
    ::  +talk: receive request to change shrub
    ::
    ::    in order to correctly process talk, the returned shrub must have all of the key-value
    ::    in add. i.e. either the write succeded or didn't
    ++  talk
      |~  add=tale:t
      *(quip card tale:t)
    ::  +take: receive notification of child change
    ++  take
      |~  =gift
      *(quip card tale:t)
    ::  +dead: notification of dependency death
    ::
    ::    must reinject with new valid dependency if the dead dep is required and writeable.
    ::    if not, the shrub's subtree will be "frozen", until some intervention that revives
    ::    the dependency.
    ++  dead
      |~  =slot:t
      *(quip card tale:t)
    ::
    ++  hear
      |~  =rely
      *(quip card tale:t)


    ::  +wack: IO to a dependency failed
    ::
    ::    it's unlikely that you would ever want do anything complex with this arm, except for
    ::    reflection/metaprogramming reasons. Included mostly for completeness.
    ::    TODO: show how semantics of shrub in practice are mostly total
    ++  goof
      |~  [c=card err=(unit (list tank))]
      *(quip card tale:t)
    --

  +$  bale  [lede=cord info=(list cord)]
  +$  sect  (list pica)
  +$  pica  cord
  ++  bill
    |$  [contents]
    [doc=bale con=contents]
  ++  mist
    $%  [%state studs=(list (bill stud:t))]
        [%poke studs=(list (bill stud:t))]
        [%kids kids=(bill port)]
        [%deps deps=(list (bill (pair term fief)))]
        [%form =hoon]
    ==
  ++  imports
    $+  imports
    $:  a=(map term path)
        b=(list [term path])
    ::    c=(list [term stud pail])
    ==
  +$  spec
    $+  spec
    $:  state=(bill curb)
        poke=(bill (set (bill stud:t)))
        kids=(unit (bill soma))
        deps=(bill (map term (bill fief)))
        =imports
    ==
  ++  bush  (pair spec form)
  ::
  ::  +kook: General purpose shrub
  +$  kook  bush
::    $_  ^&
::    |%
::    ::  $state: the state of this value in the urbit namespace
::    ::
::    ::    For instance, a message would be
::    ::    ```hoon
::    ::    [author=ship time-sent=time message=txt]
::    ::    ```
::    ::
::    ::    ```
::    ++  state  *curb
::    ::  $poke: a poke is a request to change a value in teh urbit
::    ::  namespace.
::    ::
::    ::    For instance a blocked list that is a set of users would be
::    ::      [%add who=user]
::    ::      [%del who=user]
::    ::
::    ::
::    ++  poke   *(set stud)
::    ++  form   *^form
::    ::
::    ::  +kids: Some nodes in the namespace define what children are
::    ::  allowed to be under them. For instance, it should not  be allowed
::    ::  to create /~hastuc-dibtux/chats/unit-731/blog-post-1. This is
::    ::  nonsensical because blog posts don't go in chats.
::    ++  kids   *(unit soma)
::    ::
::    ::  +deps: Some nodes in the namespace might like to hear about other
::    ::  things that happen in the namespace. For instance, a substack-type
::    ::  software would like to know where the wallet software is located
::    ::  in the name
::    ++  deps   *(map term fief)
::    --
  --


++  pec
  |%
  ++  band
    |=  s=spec:n
    %-  ~(gas by *band:n)
    %+  turn  ~(tap by con.deps.s)
    |=  [=term f=(bill:n fief:n)]
    [term con.f]
  ++  poke
    |=  s=spec:n
    (~(gas in *(set stud:t)) (turn ~(tap in con.poke.s) tail))
  ++  state
    |=  s=spec:n
    con.state.s
  ++  lash
    |=  s=spec:n
    [(state s) (poke s)]
  ++  lashes
    |=  s=spec:n
    =/  res  (kids s)
    res(fil `(lash s))
  ++  kids
    |=  s=spec:n
    ^-  soma:n
    ?~  kids.s
      *soma:n
    con.u.kids.s
  --
+$  trace-level  ?(%slog %mean %none)

++  trace-lvl
  ^-  trace-level
  %slog
++  trace-move
  |=   print=(trap tape)
  (trace 1 |.("move: {$:print}"))
++  trace
  |=  [pri=@ud print=(trap tape)]
  ^+  same
  =/  lvl=trace-level  trace-lvl
  ?-  lvl
      %none  same
      %slog
    ~>  %slog.[pri leaf/$:print]
    same
  ::
      %mean
    ~>  %mean.|.(leaf/$:print)
    same
  ==
++  prelude  !>(..n)

++  pull-lash
  |=  s=spec:n
  ^-  lash:n
  [con.state.s (~(run in con.poke.s) tail)]
++  pull-band
  |=  s=spec:n
  ^-  band:n
  (~(run by con.deps.s) tail)

++  v  vine:t
++  d  dust:t
::  nock 12 cache
+$  lice
  (map hunt:t epic:t)
++  worm
  $:  =toil:d  :: pending updates
      :: =epic:t  :: cached prev state
      =lice
      next=(list flue:n) :: "hyperparent stack"
      up=(list move:n)   :: moves to non-children
      down=(list flue:n)  :: pending moves for children
      prize=(axal mode:t)
      gifts=(list [p=pith:t q=gift:n])
      start=pith:t
      src=pith:t
      dst=pith:t
      =bowl:v
  ==
++  bore
  |_  worm
  +*  wor  +<
  ++  start
    |=  [=bowl:v =clue:n]
    ^-  worm
    =/  flues  (clue-to-flues:n clue)
    ?>  ?=(^ flues)
    =/  c  i.flues
    ::~&  start/[first=c rest=t.flues]
    %*  .  *worm
      next  t.flues
      down  [c ~]
      dst  p.q.c
      src  p.c
      start  p.q.c
      bowl  bowl
    ==
  ++  get-kooks
    |=  =tale:t
    ^-  [(list kook:n) worm]
    =/  kok-pith=(list pith:t)  ~(kook sug tale)
    =|  koks=(list kook:n)
      ::?:  =(~ koks)  ~[[*spec:n def:n]]
      ::[[koks ~] wor]
    |-  ^-  [(list kook:n) worm]
    =*  loop  $
    ?~  kok-pith
      :_  wor
      =/  default=kook:n  [*spec:n def:n]
      ^-  (list kook:n)
      ?:(=(~ koks) ~[default] (flop koks))
    =^  k=kook:n  wor  (get-kook i.kok-pith)
    loop(koks [k koks], kok-pith t.kok-pith)
  ++  read-x-latest
    |=  pax=pith:t
    ^-  [(unit saga:t) worm]
    =^  =epic:t  wor
      (read-latest %x pax)
    [fil.epic wor]

  ++  read-x
    |=  pax=pith:t
    ^-  [(unit tale:t) worm]
    =^  =muck:t  wor
      (read %x pax)
    ?~  fil.muck
      [~ wor]
    [fil.muck wor]
  ++  read-x-need
    |=  pax=pith:t
    ^-  [tale:t worm]
    =^  =muck:t  wor
      (read %x pax)
    =/  =tale:t  (need fil.muck)
    [tale wor]

  ::
  ++  get-kook
    |=  =pith:t
    ^-  [kook:n worm]
    ~|  kook/pith
    ?:  =(#/sys/boot pith)
      [[*spec:n strap-build:n] wor]
    =^  tal=tale:t  wor  (read-x-need pith)
    =/  pal=pail:t  (need ~(vase sug tal))
    ?.  ?=([%vase *] pal)
      ?:  ?=([%tang *] pal)
        (salt +.pal)
      !!
    =/  vax=vase  +:pal
::    ~_  (sell vax)
    ?:  =(stud %noun)
      [[*spec:n def:n] wor]
    ?:  =(q.vax ~)
      [[*spec:n def:n] wor]
    ~|  failed-zapgal+pith
::    ~_  (sell vax)
    :_  wor
    :-  *spec:n
    !<(form:n vax)

  ++  read-kook
    |=  =tale:t
    ^-  [kook:n worm]
    =/  kok-pith  ~(kook sug tale)
    ?~  kok-pith  [[*spec:n def:n] wor]
    (get-kook i.kok-pith)
  ::
  ++  read-lash
    |=  =pith:t
    ^-  [(unit lash:n) worm]
    =^  tal=(unit tale:t)  wor
      (read-x pith)
    ?~  tal
      [~ wor]
    =^  kok=kook:n  wor  (read-kook u.tal)
    [`(pull-lash p.kok) wor]
  ::
  ++  read-soma
    |=  [=care:t =pith:t]
    ^-  soma:n
    *soma:n
    ::=/  paths
    ::  %+  turn  ~(tap of (read care pith))
    ::  |=  [p=pith:t =tale:t]
    ::  (welp pith p)
    ::%+  roll  paths
    ::|=  [=pith:t out=soma:n]
    ::^+  out
    ::?~  her=(read-lash pith)
    ::  out
    ::(~(put ox out) (home:rail:aire pith) u.her)
  ++  apply-deltas
    |=  [hun=hunt:t =muck:t]
    ^+  muck
    =/  deltas=(list [=pith:t =note:d])
      ~(tap of (sorge care.hun (~(dip of toil) pith.hun)))
    |-
    ?~  deltas  muck
    =/  [pax=pith:t =note:d]  i.deltas
    %_  $
        deltas  t.deltas
        muck
      ?-  -.note
        %make  (~(put of muck) pax tale.note)
        %poke  (~(jab of muck) pax |=(=tale:t (~(uni by tale) tale.note)))
        %cull  (~(del of muck) pax)
      ==
    ==
  ::
  ++  read
    |=  [=care:t =pith:t]
    ^-  [muck:t worm]
    ?^  new=(~(get by lice) [care pith])
      [(strip u.new) wor]
    =^  =epic:t  wor
      (read-latest care pith)
    :_  wor
    (apply-deltas [care pith] (strip epic))
  ::
  ++  read-latest
   |=  [=care:t =pith:t]
   ^-  [epic:t worm]
   =/  =epic:t  (peek-rom care pith)
   =.  lice  (~(put by lice) [care pith] epic)
   [epic wor]
  ++  remove
    |=  =pith:t
    =.  toil  (~(put of toil) pith cull/~)
    =.  prize  (~(put of prize) pith %del)
    wor
  ++  pop-gift
    ^-  [(unit sign:n) worm]
    ?~  gifts  `wor
    =/  =flue:n  [p.i.gifts %take p.i.gifts q.i.gifts]
    =.  now.bowl  +(now.bowl)
    =>  .(gifts `(list [pith:t gift:n])`t.gifts)
    (pop flue)

  ++  pop-down
    ^-  [(unit sign:n) worm]
    ?~  down  `wor
    =.  now.bowl  +(now.bowl)
    =/  m  i.down
    =>  .(down `(list flue:n)`t.down)
    (pop m)
  ++  print-card
    |=  =card:n
    ^-  tang
    :~  leaf+"dst:{<card>}"
    ==
  ++  print-move
    |=  =move:n
    ^-  tang
    :-  leaf+"src:{<p.move>}"
    (print-card q.move)
  ++  print-worm
    ^-  tang
    *tang
::  %-  zing
::  %-  zing
::  :~  `(list tang)`[leaf+"up ---------------------------------------" ~]^~
::      `(list tang)`(turn up print-move)
::      `(list tang)`[leaf+"next ---------------------------------------" ~]^~
::      `(list tang)`(turn next print-move)
::      `(list tang)`[leaf+"down ---------------------------------------" ~]^~
::      `(list tang)`(turn down print-move)
::  ==
  ++  pop
    |=  =flue:n
    ^-  [(unit sign:n) worm]
    =:  src  p.flue
        dst  (get-true-pith:n q.flue)
      ==
    =/  v=(unit view:t)
      ?.  ?=([%over *] dst)  ~
      (get-view dst)
    =?  dst       ?=(^ v)
      pith.hunt.u.v

    ::=?  q.q.clue  ?=(^ v)
    ::  ?~  fun=(peek-case case.over.u.v %x pith.over.u.v)
    ::    q.q.clue
    ::  ?~  fil.u.fun  q.q.clue
    ::  =/  con=tale:t  (~(got by q.u.fil.u.fun) content:slot:t)
    ::  ?>  ?=([%dish *] con)
    ::  =/  =dish:t  +.con
    ::  q.q.clue
      :: TODO: why broken?
      ::=/  back  (need back.dish)
      ::?+  -.q.q.clue   ~|(wack-lens/-.q.q.clue !!)
      ::  %poke  q.q.clue(tale (back tale.q.q.clue))
      ::  %make  q.q.clue(tale (back tale.q.q.clue))
      ::==
    ::%-  (trace-move |.("src: {(en-tape:pith:t src)} dst: {(en-tape:pith:t dst)}"))
    ::%-  %-  trace-move
        ::|.
        ::?+  -.q.q.clue  (trip -.q.q.clue)
         :: %poke  [%poke p.pail.q.q.move]
         ::%poke   "poke {<~(key by tale.q.q.clue)>}"
         ::%make   "make {<~(key by tale.q.q.clue)>}"
         ::%many  "many len:{(scow %ud (lent cards.q.q.move))}"
        ::==

::    ~&  [src=src dst=dst]
::    ~&  ^=  note
::        ?+  -.q.q.move  -.q.q.move
::          %poke  [%poke p.pail.q.q.move]
::          %make  [%make code.q.q.move]
::        ==
        ::?:  ?=(%poke -.q.q.move)  [%poke p.pail.q.q.move]

    [`(true-to-sign:n q.flue) wor]
  ++  pop-next
    ^-  [(unit sign:n) worm]
    ?~  next  `wor
    =/  m  i.next
    ~&  %pop-next
    =.  now.bowl  +(now.bowl)
    =>  .(next `(list flue:n)`t.next)
    ?>  =(~ down)
    (pop m)
  ++  enqueue
    |=  ome=(list omen:n)
    ^+  wor
    =/  trues=(list true:n)
      (zing (turn ome omen-to-true:n))
    %_    wor
        next
      %+  welp  next
      ^-  (list flue:n)
      (turn trues (lead src))
    ==
  ::
  ::++  grow
  ::  |=  [=pith:t =tale:t]
  ::  ^-  worm
  ::  ?~
  ::  ?:  (~(has of muck) pith)
  ::    (poke pith tale)
  ::  (make pith tale)
  ++  poke
    |=  [=pith:t =tale:t]
    ^-  worm
    =^  old=(unit tale:t)  wor  (read-x pith)
    =/  changes=note:dust
      (~(gut of toil) pith [%poke *tale:t])
    =/  =mode:t  %dif
    =.  changes
      ?-  -.changes
        %cull   ~&  %wack  [%poke *tale:t]
        %poke  changes(tale tale)
        %make  changes(tale tale)
      ==
    =.  toil  (~(put of toil) pith changes)
    =.  prize  (~(put of prize) pith mode)
    wor
  ::
  ++  make
    |=  [=pith:t =tale:t]
    ^-  worm
    =^  old=(unit tale:t)  wor  (read-x pith)
    ?:  &(?=(^ old) =(u.old tale))
      wor
    =/  =mode:t  %add
    ::
    ::=.  muck  (~(put of muck) pith tale)
    =.  toil  (~(put of toil) pith make/tale)
    =.  prize  (~(put of prize) pith mode)
    wor
  ::
  ++  husk
    |_  =stud:t
    ::
    ++  pith
      ^-  pith:t
      (stud-to-pith:t stud %imp)
    ++  get-pail
      =/  tal=(unit tale:t)  fil:(read %x pith)
      ^-  (unit pail:t)
      ?~  tal  ~
      (~(get by u.tal) vase:slot:t)
    ++  has
      =/  pal  get-pail
      ?~  pal
        |
      !=(%tang -.u.pal)
    ::
    ++  vase
      ^-  ^vase
      ~|  husk/stud
      =/  pal  (need get-pail)
      ~!  pal
      ?>  ?=(%vase -.pal)
      +.pal
    ++  is-bunted
      =(~ +:vase)
    --
  --
--
=>
::
::  adult core
|_  =bowl:v
++  adult-core  .
++  neon  .
++  this  .
::
++  plow
  |=  =clue:n
  ^-  fate:v
  =<  abet:work:abed
  |_  w=worm
  +*  bor  ~(. bore w)
  ++  abet
    ^-  fate:v
    [~(aap of toil.w) up.w ~]
    ::=.  plow  deal
    ::[(check-builds toil.w) this]
::    ~&  %abet
::    =.  this  deal
::    =/  toil  (check-builds toil.w)
::    ?~  tol=(~(get of toil) /sys/vine)
::      this
::    ?>  ?=(%make -.u.tol)
::    q.q.u.tol
::    [(check-builds toil.w) this]:deal
  ++  plow  .
  ++  abed
    plow(w (start:bor bowl clue))
  ::
  ::++  lift-pail
  ::  |=  =pail:t
  ::  ^-  vase
  ::  ::(slop !>(p.pail) q.pail)
  ++  work
    ^+  plow
    =^  nex=(unit sign:n)  w  pop-down:bor
    ?~  nex  plunder
    (apply u.nex) :: XX: weird compiler?
  ++  apply
    |=  =sign:n
    ^+  plow
    ?+   -.sign  ~|(-.sign !!)

      %hear  (hear +.sign)
      %make  (make +.sign)
      %poke  (poke +.sign)
      %take  (take +.sign)
      %cull  cull
      ::  %many
      :::: ?~  cards.note  plow
      ::=.  w  (enqueue:bor cards.sign)
      ::=^  not=(unit sign:n)  w  pop-next:bor
      ::?~  not  plow
      ::(apply u.not)
    ==
  ++  hear
    |=  =rely:n
    =/  tal=tale:t  (need fil:(read:bor %x dst.w))
    :: ~>  %bout.[1 (spat %hear dst.w)]
    =/  [cards=(list card:n) new=tale:t]
      (hear:(kook-here tal) rely)
    =.  w
      (poke:bor dst.w new)
    (ingest cards)
  ++  take
    |=  =gift:n
    =/  tal=tale:t  (need fil:(read:bor %x dst.w))
    ~|  talk-kook-here+~(key by tal)

    ::  ~>  %bout.[1 (spat %take dst.w)]
    =/  [cards=(list card:n) new=tale:t]
      (take:(kook-here tal) gift)
    =.  w
      (poke:bor dst.w new)
    (ingest cards)
  ::  +plunder: emptied downward stack
  ::
  ::    gifts to give, do that, else start on next hyperparent if any
  ::
  ++  plunder
    ^+  plow
    ?.  =(0 ~(wyt of prize.w))
      raid
    =^  n=(unit sign:n)  w  pop-next:bor
    ?~  n  plow
    (apply u.n)
  ::
  :: +raid: Fill gift stack and restart flow
  ::
  ++  adopt

    |=  =pith:t
    ^-  (set pith:t)
    ?~  par=(peek-parent pith)
      ~
    (sy u.par ~)
  ++  raid
    :: XX: maybe we should materialize this as we go
    ^+  plow
    =/  by-parent=(jug pith:t [=pith =mode:t])
      :: ~>  %bout.[1 %by-parent]
      %+  roll  ~(aap of prize.w)
      |=  [[=pith:t =mode:t] out=(jug pith:t [=pith =mode:t])]
      %-  ~(gas ju out)
      (turn ~(tap in (adopt pith)) |=(par=pith:t [par [(dif:pith:t par pith) mode]]))
    =.  down.w
      :: ~>  %bout.[1 %new-gifts]
      %+  welp  down.w
      ~
      ::%+  turn  (sort ~(tap in ~(key by by-parent)) sort:pith:t)
      ::|=  =pith:t
      ::^-  clue:n
      ::*clue:n
      ::[pith pith %take (~(gas of *gift:n) ~(tap in (~(get ju by-parent) pith)))]
    =.  prize.w  *(axal mode:t)
    work
  ::
  ::  +give: Pop off gift stack
  ::
  ++  give
    ^+  plow
    =^  n=(unit sign:n)  w  pop-gift:bor
    ?~  n  plunder
    (apply u.n)
  ++  ingest
    |=  caz=(list card:n)
    =/  =pith:t  dst.w
    :: ~&  pith/pith
    =.  up.w
     %+  welp  up.w
     ^-  (list move:n)
     %+  murn  caz
     |=  =card:n
     ^-  (unit move:n)
     ?:  (card-is-down:n pith card)
       ~
    :: XX: requires that downward batches and upward batches are never
    :: mixed, should probably enforce
     `[pith card]
    =.  down.w
      %+  welp  down.w
      %-  zing
      %+  turn  caz
      |=  =card:n
      ^-  (list flue:n)
      %+  murn  (card-to-lards:n card)
      |=  =lard:v
      ^-  (unit flue:n)
      ?.  (is-parent:u pith p.lard)
        ~
      `[pith lard]

    work
  ::
  ++  make
    |=  init=tale:t
    ^+  plow
    =/  prev  fil:(read:bor %x dst.w)
::  =/  prev-wave  fil:(read:bor %x %int dst.w)
    ::
    =^  koks=(list kook:n)  w   (get-kooks:bor init)
    =|  cards=(list card:n)
    ::  ~>  %bout.[1 (spat %make dst.w)]
    |-
    ?~  koks
      ::=/  ome=(list omen:n)
      ::  %+  turn  cards
      ::  |=  =card:n
      ::  ^-  omen:n
      ::  card
      =.  w
        :: ?:  &(?=(^ prev) (is-tale-super u.prev new))
          :: ~&  %skipping   w
        (make:bor dst.w init)
      (ingest cards)
    =/  =band:n   (pull-band p.i.koks)
    ~&  dst/dst.w
    =/  =crew:n   ~(crew sug init)
    ~>  %slog.[0 (crip ~(ram re >crew<))]
    =/  flaw      (dock crew band)
    ::=.  plow      (listen crew band dst.w)
    =.  w         (make:bor dst.w init)
    ~|  make-kook-here+~(key by init)
    =^  caz=(list card:n)  init
      (init:(hydrate-kook i.koks init) init)
    $(cards (welp cards caz), koks t.koks)
  ::
  ++  listen
    |=  [=crew:n =band:n for=pith:t]
    ^+  plow
    =/  crew  ~(tap by crew)
    |-  ?~  crew  plow
    =/  [=term =pith:t]  i.crew
    ~|  pith/pith
    ?:  =(%over -.pith)  plow
    =^  =muck:t  w  (read:bor %x pith)
    =/  =tale:t  (need fil.muck)
    ::  =+  !<(=wave:n q.q.tale)
    =/  fef=(unit fief:n)  (~(get by band) term)
    ::=+  ;;(care=?(%x %y %z) care.fief)
::    =/  fans  ~(fans sug tale)
::    =.  fans  (~(put ju fans) care for)
::    =.  tale  (~(put by tale) fans:slot:t fans/!>(fans))
::    =.  w  (grow:bor pith tale)
    $(crew t.crew)
  ::
  ++  reign
    =|  res=(map term (pair pith:t epic:t))
    |=  [=kook:n =crew:n]
    ^+  res
    ~|  reign-fail+dst.w
    %-  ~(gas by res)
    %+  murn  ~(tap by crew)
    |=  [=term =pith]
    ^-  (unit [^term [^pith epic:t]])
    =/  =fief:n
      ?^  fef=(~(get by (pull-band p.kook)) term)
        u.fef
      [[~ |] %x *soma:n]
    ~&  pith/pith
    =/  pic=epic:t  (peek-rom care.fief pith)
    ::?~  pic  ~
    `[term [pith pic]]
  ::
  ++  kook-here
    |=  =tale:t
    =^  =kook:n  w  (read-kook:bor tale)
    (hydrate-kook kook tale)

  ++  hydrate-kook
    |=  [=kook:n =tale:t]
    ^-  [form:n worm]
    =^  =muck:t  w  (read:bor %y dst.w)
    =/  cup=bowl:n
      :*  src.w
          dst.w
          now.bowl.w
          `@uv`0xdead.beef
          (reign kook ~(crew sug tale))
          muck
      ==
    =^  sag=(unit saga:t)  w  (read-x-latest:bor dst.w)
    =/  =saga:t
      ?^  sag  u.sag(q tale)
      ~&  %lost-saga
      =|  =saga:t
      saga(q tale)
    :_  w
    ~(. q:kook [cup saga])
  ::
  ::  ++  make
::    |=  init=tale:t
::    ^+  plow
::    =/  prev  fil:(read:bor %x dst.w)
::::  =/  prev-wave  fil:(read:bor %x %int dst.w)
::    ::
::    =/  koks=(list kook:n)   (read-kooks:bor init)
::    =|  cards=(list card:n)
::    |-
::    ?~  koks
::      =/  ome=(list omen:n)
::        %+  turn  cards
::        |=  =card:n
::        ^-  omen:n
::        card
::      =.  w
::        ?:  &(?=(^ prev) (is-tale-super u.prev new))
::          ~&  %skipping   w
::        (make:bor dst.w new)
::      (ingest ome)
::    =/  =band:n   (pull-band i.koks)
::    =/  =crew:n   ~(crew sug init)
::    =/  flaw      (dock crew band)
::    =.  plow      (listen crew band dst.w)
::    =.  w         (make:bor dst.w init)
::    ~|  make-kook-here+~(key by init)
::    =^  caz=(list card:n) init
::      (init:(hydrate-kook i.koks) init)
::    $(cards (welp cards caz), koks t.koks)
  ::  shrines
  ::    (list notifications) -> (list updates)
  ::    :: async, merge semantics
  ::    :: lawful to merge double notification
  ::    :: nacks are advisory and only delivered to the dependent shrub
  ::    :: /foo -> /bar (crashes)
  ::    :: crash -> /bar
  ::    (list commands) -> (list updates)
  ::    :: sync, in-order
  ::    :: nacks must be delivered at the runtime layer
  ::    :: /foo -> /bar (crashes)
  ::    :: crash -> /foo
  ::    ::  query: path -> namespace
  ::    ::    runtim
  ::
  ::
  ++  poke
    |=  pok=tale:t
    ^+  plow
    =^  tal=(unit tale:t)  w
      (read-x:bor dst.w)
    ~|  `*`dst.w
    =/  tal=tale:t  (need tal)
    ~|  poke-kook-here+~(key by pok)
    =^  koks=(list kook:n)  w
      (get-kooks:bor (~(uni by tal) pok))
    =|  cards=(list card:n)
    ~?  =(dst.w /demo)
      before/[tal pok]
    ~?  =(dst.w /demo)
      koks/koks
    ~&  koks/(lent koks)
    :: ~>  %bout.[1 (spat %poke dst.w)]
    |-
    ?~  koks
      ~?  =(dst.w /demo)
        done/pok
      =.  w  (poke:bor dst.w pok)
      (ingest cards)
    =^  f=form:n  w
      (hydrate-kook i.koks tal)
    =^  caz=(list card:n)  pok
      (talk:f pok)
    ~?  =(dst.w /demo)
      got/[pok tal]
    =.  tal  pok
    ::  ?>  (check-poke-valid pok new)
    $(cards (welp cards caz), koks t.koks)
  ::
  ++  check-poke-valid
    |=  [pok=tale:t new=tale:t]
    %+  levy  ~(tap by pok)
    |=  [=slot:t =pail:t]
    ^-  ?
    ?~  n=(~(get by new) slot)  |
    (is-pail-eq u.n pail)

  ::  XX: intercept cull and cleanup?
  ++  cull
    ^+  plow
    =.  w  (remove:bor dst.w)
    work
  ::
  ++  dock
    |=  [=crew:n =band:n]
    %+  roll  ~(tap in ~(key by band))
    |=  [key=term v=(unit flaw:n)]
    ^+  v
    ?.  =(~ v)  v
    =/  pax  (~(get by crew) key)
    =/  fef  (~(got by band) key)
    ?~  pax
      ?:(req.deed.fef `[key lost/~] ~)
    =/  tal  fil:(read:bor %x u.pax)
    ?~  tal
      `[key gone/~]
    =/  =soma:n  (read-soma:bor care.fef u.pax)
    ::?:  (nest:soma:aire soma soma.fef)
      ~
    ::`[key %vair ~]
  --
::
++  call
  |=  =move:v
  ^-  fate:v
  =/  =clue:n  move
  (plow clue)
::
++  hear
  |=  =news:t
  ^-  fate:v
  =/  moves  (~(dip of news) #/moves)
  =/  relies=(map pith:t rely:n)
    %+  roll  ~(tap of news)
    |=  [[changed=pith:t =word:t] out=(map pith:t rely:n)]
    %+  roll  ~(tap by for.word)
    |=  [sub=pith:t o=_out]
    ?:  &(?=(^ sub) =(i.sub %xeno))  o
    (~(put ju o) sub changed)
  =/  rel  ~(tap by relies)
  ?>  ?=(^ rel)
  =/  =clue:n
    :-  ~
    =/  start=omen:n  [%hear -.i.rel +.i.rel]
    %+  roll  t.rel
    |=  [[sub=pith:t changes=(set pith:t)] out=_start]
    ^-  omen:n
    =/  next=omen:n  [%hear sub changes]
    [out next]
  (plow clue)
++  get-pro-vase
  |=  =stud:t
  ^-  vase
  =/  pax=path
    ?@  stud  /std/pro/[stud]
    (pout stud)
  =+  .^(y=(unit epic:t) pax)
  =/  =epic:t  (need y)
  =/  =pail:t
    (~(got by q:(need fil.epic)) vase:slot:t)
  ?>  ?=(%vase -.pail)
  +.pail
::++  harden  ~(. harden:vine:t get-pro-vase)
--
=>
|%
++  poke
  |=  [now=@da eny=@uv ovo=^]
  ^-  fate:v
  =/  =task:vine
    ~>  %mean.'lain: bad task'
    :: ~>  %bout.[1 %harden-task]
    ~_  ovo/ovo
    ::~|  bad-task+[;;(path -.ovo) ;;(@t +<.ovo)]
    ;;(task:vine ovo)
  =/  =bowl:v
    [~met / now eny]
  ?-    -.task
        %vine
      =/  =move:v  [#/out p.task]
      (~(call neon bowl) move)
        %hear
      (~(hear neon bowl) news.task)
  ==
++  peek
  |=  [case=(unit @ud) =hunt:t]
  ^-  (unit epic:t)
  =/  =bowl:v
    [~met / ~2001.1.1 0v0]
  ?~  case  `(peek-rom hunt)
  (peek-case u.case hunt)
::
++  wish
  |=  txt=@t
  +:(slap !>(..neon) (ream txt))
++  load
  |=  hir=*
  ::=.  sol  hir
  [~ ..poke]
--
|=  [now=@da eny=@uv ovo=^]
^-  *
(poke now eny ovo)



