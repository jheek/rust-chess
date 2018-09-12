import React, { Component } from 'react';
import './App.css';
import Chess from "./react-chess";

class App extends Component {

  constructor() {
    super()
    this.state = {
      lineup: Chess.getDefaultLineup(),
      legalMoves: [],
      autoplayBlack: false,
      autoplayWhite: false,
    };

    this.openConnection();
  }

  componentWillUnmount() {
    console.log("unmount");
    this.ws.close();
  }

  openConnection = () => {
    if (this.ws) {
      this.ws.close();
    }
    this.ws = new WebSocket("ws://localhost:3012");
    this.ws.addEventListener("message", this.handleMessage);
    this.ws.addEventListener("open", this.handleOpen);
    this.ws.addEventListener("close", this.handleClose);
  }

  isLegalMove = (from, to) => {
    return this.state.legalMoves.some(({from: legalFrom, to: legalTo}) => (legalFrom == from && legalTo == to));
  }

  handleOpen = (event) => {
    console.log("open");
    this.ws.send(JSON.stringify("Reset"));
  }

  handleMessage = (ev) => {
    let msg = JSON.parse(ev.data);
    console.log(msg);
    let autoplay = this.state[msg.side_to_move == "white" ? "autoplayWhite" : "autoplayBlack"];
    if (autoplay) {
      let line = msg.best_line;
      if (line.length > 0) {
        let move = line[0];
        this.ws.send(JSON.stringify({"Move": move}));
      }
    }
    this.setState((state) => ({
      ...state,
      sideToMove: msg.side_to_move,
      legalMoves: msg.legal_moves,
      lineup: msg.lineup,
      bestLine: msg.best_line,
      bestValue: msg.best_value,
    }));
  }

  handleClose = (ev) => {
    console.log("close", ev);
    setTimeout(this.openConnection, 3000);
  }

  handleMovePiece = (piece, from, to) => {
    this.setState((state) => {
      let lineup = this.state.lineup;
      let newLineup = [...lineup];
      if (this.isLegalMove(from, to)) {
        newLineup = this.state.lineup.map((x) => x.replace(from, to));
        this.ws.send(JSON.stringify({"Move": {from, to}}));
      } else {
        console.log("illegal move!", from, to, this.state.legalMoves);
      }
      return {...state, lineup: newLineup};
    });
  }

  handleInputChange = (event) => {
    const target = event.target;
    const value = target.type === 'checkbox' ? target.checked : target.value;
    const name = target.name;

    this.setState((state) => ({
      ...state,
      [name]: value
    }));
  }

  render() {
    let focusTiles = [];
    let line = this.state.bestLine;
    let bestLineTxt = "";
    if (line && line.length > 0) {
      focusTiles = [line[0].from, line[0].to];
      bestLineTxt = this.state.bestValue + " " + line.map(({from, to}) => `${from}-${to}`).join(" ");
    }
    console.log("bestLineTxt", bestLineTxt)
    return (
      <div>
        <div className="Chess">
          <Chess pieces={this.state.lineup} onMovePiece={this.handleMovePiece} focusTiles={focusTiles} />
        </div>
        <pre>
          {bestLineTxt}
        </pre>
        <form>
          <label>
            <input
              name="autoplayWhite"
              type="checkbox"
              checked={this.state.isAutoPlayWhite}
              onChange={this.handleInputChange} />
            AutoPlay White
          </label>
          <br/>
          <label>
            <input
              name="autoplayBlack"
              type="checkbox"
              checked={this.state.isAutoPlayBlack}
              onChange={this.handleInputChange} />
            AutoPlay Black
          </label>
        </form>
      </div>
    );
  }
}

export default App;
