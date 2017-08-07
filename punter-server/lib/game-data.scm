(define-module (lib game-data)
  #:use-module (srfi srfi-9)
  #:use-module (srfi srfi-42)
  #:export (game-state
            game-state?
            make-game-state
            game-state-moves
            add-game-state-move!
            game-state-claims
            game-state-futures
            game-state-options

            game
            game?
            make-game
            game-punters-count
            game-game-map
            punters-list

            game-map
            game-map?
            make-game-map
            game-map-sites
            game-map-mines
            game-map-rivers

            river
            river?
            make-river
            river-source
            river-target

            future
            future?
            make-future
            future-source
            future-target

            site
            site?
            make-site
            site-id
            site-x
            site-y

            pass-move
            claim-move
            splurge-move
            option-move))

(define-record-type game-state
  (_make-game-state moves claims futures options)
  game-state?
  (moves game-state-moves set-game-state-moves!)
  (claims game-state-claims)
  (futures game-state-futures)
  (options game-state-options))

(define-record-type game
  (make-game punters-count game-map)
  game?
  (punters-count game-punters-count)
  (game-map game-game-map))

(define-record-type game-map
  (make-game-map sites mines rivers)
  game-map?
  (sites game-map-sites)
  (mines game-map-mines)
  (rivers game-map-rivers))

(define-record-type river
  (make-river source target)
  river?
  (source river-source)
  (target river-target))

(define-record-type future
  (make-future source target)
  future?
  (source future-source)
  (target future-target))

(define-record-type site
  (make-site id x y)
  site?
  (id site-id)
  (x site-x)
  (y site-y))

(define (pass-move punter)
  `((pass . ((punter . ,punter)))))

(define (claim-move punter source target)
  `((claim . ((punter . ,punter)
              (source . ,source)
              (target . ,target)))))

(define (splurge-move punter route)
  `((splurge . ((punter . ,punter)
                (route . ,route)))))

(define (option-move punter source target)
  `((option . ((punter . ,punter)
               (source . ,source)
               (target . ,target)))))

(define (punters-list punters-count)
  (list-ec (:range i punters-count) i))

(define (make-game-state punters-count)
  (_make-game-state
   (map
    pass-move
    (punters-list punters-count))
   (make-hash-table)
   (make-hash-table)
   (make-hash-table)))

(define (add-game-state-move! game-state move)
  (set-game-state-moves! game-state (cons move (game-state-moves game-state))))

